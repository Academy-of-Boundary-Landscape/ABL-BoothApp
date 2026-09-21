//! 数据库快照与恢复。
//!
//! ② 之后每一次 schema 变更都靠这里兜底：迁移前落一份事务一致的副本，
//! 迁移失败就还原，不让应用带着半迁移的库启动。

use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

/// 保留的快照份数。更旧的在每次 take 之后删掉，避免无限堆积。
pub const KEEP: usize = 3;

fn bak_prefix(db_path: &Path) -> String {
    let stem = db_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "sale_system.db".to_string());
    format!("{stem}.bak-")
}

/// 用 `VACUUM INTO` 落一份事务一致的快照。
///
/// **必须是 VACUUM INTO 而不是 fs::copy**：库跑在 WAL 模式下，直接拷 .db
/// 会漏掉 -wal 中尚未 checkpoint 的数据，拷出来可能是残缺的。
pub async fn take(pool: &SqlitePool, db_path: &Path, tag: &str) -> Result<PathBuf, sqlx::Error> {
    let stamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let name = format!("{}{}-{}", bak_prefix(db_path), tag, stamp);
    let dest = db_path.with_file_name(name);

    // VACUUM INTO 要求目标文件不存在
    if dest.exists() {
        std::fs::remove_file(&dest).ok();
    }

    // VACUUM INTO 不接受绑定参数，只能拼字符串；转义单引号防止路径里的引号破坏语句。
    // 这段拼接过的 SQL 已经过转义审计，用 AssertSqlSafe 告诉 sqlx 这是有意为之。
    let escaped = dest.to_string_lossy().replace('\'', "''");
    sqlx::query(sqlx::AssertSqlSafe(format!("VACUUM INTO '{escaped}'")))
        .execute(pool)
        .await?;

    prune(db_path, KEEP);
    Ok(dest)
}

/// 用快照覆盖主库。调用前 pool 必须已经关闭。
pub fn restore(db_path: &Path, snap: &Path) -> std::io::Result<()> {
    // 过期的 -wal/-shm 必须删掉，否则会污染刚还原的库
    for suffix in ["-wal", "-shm"] {
        let side = db_path.with_file_name(format!(
            "{}{}",
            db_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            suffix
        ));
        if side.exists() {
            std::fs::remove_file(side).ok();
        }
    }
    std::fs::copy(snap, db_path)?;
    Ok(())
}

/// 只保留最新的 `keep` 份快照。文件名里的时间戳是定长的，按名字排序即按时间排序。
pub fn prune(db_path: &Path, keep: usize) {
    let Some(dir) = db_path.parent() else {
        return;
    };
    let prefix = bak_prefix(db_path);

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut baks: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().starts_with(&prefix))
                .unwrap_or(false)
        })
        .collect();

    if baks.len() <= keep {
        return;
    }

    baks.sort();
    let drop_count = baks.len() - keep;
    for p in baks.into_iter().take(drop_count) {
        std::fs::remove_file(p).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::SqlitePool;
    use std::str::FromStr;

    /// 建一个 WAL 模式的文件库，跑完迁移，插一行 settings。
    async fn wal_db(dir: &std::path::Path) -> (SqlitePool, std::path::PathBuf) {
        let db_path = dir.join("sale_system.db");
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))
            .unwrap()
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        let pool = SqlitePool::connect_with(opts).await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        sqlx::query("INSERT INTO settings (key, value) VALUES ('probe', 'v1')")
            .execute(&pool)
            .await
            .unwrap();
        (pool, db_path)
    }

    #[tokio::test]
    async fn take_produces_readable_consistent_copy() {
        let dir = tempfile::tempdir().unwrap();
        let (pool, db_path) = wal_db(dir.path()).await;

        let snap = take(&pool, &db_path, "premigrate").await.unwrap();
        assert!(snap.exists(), "快照文件应存在: {snap:?}");

        // 关键：不 checkpoint 直接打开快照，也必须读得到刚写入的那一行。
        // 这正是 fs::copy 会失败而 VACUUM INTO 能通过的地方。
        let snap_pool = SqlitePool::connect(&format!("sqlite://{}", snap.display()))
            .await
            .unwrap();
        let v: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'probe'")
            .fetch_one(&snap_pool)
            .await
            .unwrap();
        assert_eq!(v, "v1");
    }

    #[tokio::test]
    async fn prune_keeps_only_the_newest_n() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sale_system.db");
        std::fs::write(&db_path, b"").unwrap();

        for stamp in [
            "20260101000001",
            "20260101000002",
            "20260101000003",
            "20260101000004",
        ] {
            std::fs::write(
                dir.path()
                    .join(format!("sale_system.db.bak-premigrate-{stamp}")),
                b"",
            )
            .unwrap();
        }
        // 无关文件不该被碰
        std::fs::write(dir.path().join("unrelated.txt"), b"").unwrap();

        prune(&db_path, 2);

        let mut left: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".bak-"))
            .collect();
        left.sort();
        assert_eq!(
            left,
            vec![
                "sale_system.db.bak-premigrate-20260101000003",
                "sale_system.db.bak-premigrate-20260101000004"
            ]
        );
        assert!(dir.path().join("unrelated.txt").exists());
        assert!(db_path.exists(), "主库不能被 prune 删掉");
    }

    #[tokio::test]
    async fn restore_replaces_db_and_clears_wal_sidecars() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("sale_system.db");
        let snap = dir.path().join("snap.db");
        std::fs::write(&db_path, b"broken").unwrap();
        std::fs::write(dir.path().join("sale_system.db-wal"), b"stale").unwrap();
        std::fs::write(dir.path().join("sale_system.db-shm"), b"stale").unwrap();
        std::fs::write(&snap, b"good").unwrap();

        restore(&db_path, &snap).unwrap();

        assert_eq!(std::fs::read(&db_path).unwrap(), b"good");
        // 旧的 -wal/-shm 必须删掉，否则新库会被过期的 WAL 污染
        assert!(!dir.path().join("sale_system.db-wal").exists());
        assert!(!dir.path().join("sale_system.db-shm").exists());
    }
}
