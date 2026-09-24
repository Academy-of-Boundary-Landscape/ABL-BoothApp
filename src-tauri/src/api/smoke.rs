//! 真实数据冒烟：拿一份真实的库（的副本）跑遍所有 GET 路由，断言没有 5xx。
//!
//! 夹具造不出来的历史数据（v1.0 时代的空字段、手工改过的行）只有真库里有。
//! 默认 `#[ignore]`，手动跑：
//!
//! ```bash
//! BOOTH_SMOKE_DB=~/.local/share/com.abl.BoothKernel-dev/sale_system.db \
//!   tauri-env linux cargo test --all-features real_db_smoke -- --ignored --nocapture
//! ```
//!
//! 库文件先被**复制**到临时目录再打开（会跑迁移），原文件不受影响。

#[cfg(test)]
mod tests {
    use crate::test_support::{admin_token, json_request, TEST_JWT_SECRET};
    use axum::Router;
    use serde_json::json;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    #[ignore = "需要 BOOTH_SMOKE_DB 指向一份真实的库"]
    async fn real_db_smoke() {
        let src = std::env::var("BOOTH_SMOKE_DB").expect("设置 BOOTH_SMOKE_DB");
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("smoke.db");
        std::fs::copy(&src, &db_path).expect("复制库文件");
        let upload_dir = dir.path().join("uploads");
        std::fs::create_dir_all(&upload_dir).unwrap();

        let opts =
            SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display())).unwrap();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.expect("迁移真实库");

        let state = crate::state::AppState {
            db: pool.clone(),
            upload_dir: upload_dir.clone(),
            app_data_dir: dir.path().to_path_buf(),
            jwt_secret: TEST_JWT_SECRET.to_string(),
            vision_runtime: Arc::new(crate::vision::VisionRuntime::new(
                dir.path().to_path_buf(),
                upload_dir,
                pool.clone(),
            )),
        };
        let (api, doc) = crate::api::router().split_for_parts();
        let router = Router::new().nest("/api", api).with_state(state);

        // 路径参数取真实库里的第一个 id
        let first = |sql: &'static str| {
            let pool = pool.clone();
            async move {
                sqlx::query_scalar::<_, i64>(sql)
                    .fetch_optional(&pool)
                    .await
                    .unwrap()
            }
        };
        let event_id = first("SELECT id FROM events ORDER BY id LIMIT 1").await;
        let order_id = first("SELECT id FROM orders ORDER BY id LIMIT 1").await;
        let token = admin_token();

        let mut checked = 0;
        let mut skipped = Vec::new();
        let mut failures = Vec::new();
        for (path, item) in doc.paths.paths.iter() {
            if item.get.is_none() {
                continue;
            }
            let mut uri = format!("/api{path}");
            let mut ok = true;
            for (name, value) in [("event_id", event_id), ("order_id", order_id)] {
                let key = format!("{{{name}}}");
                if uri.contains(&key) {
                    match value {
                        Some(v) => uri = uri.replace(&key, &v.to_string()),
                        None => ok = false,
                    }
                }
            }
            // `{id}` 在 /events 下就是展会 id
            if uri.starts_with("/api/events/{id}") {
                match event_id {
                    Some(v) => uri = uri.replacen("{id}", &v.to_string(), 1),
                    None => ok = false,
                }
            }
            if !ok || uri.contains('{') {
                skipped.push(path.clone());
                continue;
            }
            let res = router
                .clone()
                .oneshot(json_request("GET", &uri, Some(&token), json!(null)))
                .await
                .unwrap();
            let status = res.status();
            println!("{status} GET {uri}");
            checked += 1;
            if status.is_server_error() {
                failures.push(format!("{status} GET {uri}"));
            }
        }
        println!("checked {checked}, skipped (无法填参数) {skipped:?}");
        assert!(checked > 0, "一个路由都没跑到");
        assert!(
            failures.is_empty(),
            "真实库上出现 5xx：\n{}",
            failures.join("\n")
        );
    }
}
