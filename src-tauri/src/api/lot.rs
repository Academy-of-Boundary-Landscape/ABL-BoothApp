//! Lot = 候选商品集合 + 要选几件(N) + 总价（spec 4.1）。
//!
//! 「固定成分」是「任选 N」的特例（N = 候选集大小），模型里只有一个概念，不是两套并行。
//!
//! **单品不在这张表里。** 单品在求解器眼里是「候选集只有自己、N = 1、价格就是单价」的
//! 退化 Lot，那是求解器**输入归一化**的事，不是模型的事——管理员眼里仍然是
//! 「改这个商品的价格」（spec 4.1，路线图 D3 的「一切都是 Lot」按这个理解落地）。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, put},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;

use crate::{
    api::guard::{check_read_permission, check_write_permission},
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

/// 一个 Lot 最多几个候选商品。防的是「把全场商品一次性圈进来」这种请求，
/// 而不是业务需要——真实套装候选集十几个封顶。
const MAX_CANDIDATES: usize = 200;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events/:event_id/lots", get(list_lots).post(create_lot))
        .route(
            "/events/:event_id/lots/:lot_id",
            put(update_lot).delete(delete_lot),
        )
}

#[derive(Serialize)]
struct LotResponse {
    id: i64,
    event_id: i64,
    name: String,
    pick_count: i64,
    /// 单位：分。
    total_price: i64,
    candidate_ids: Vec<i64>,
    /// 候选集必然同一货主（下面的 `validate_candidates` 保证），所以取其一即可。
    owner_society_id: i64,
    owner_society_name: String,
}

#[derive(Deserialize)]
struct LotPayload {
    name: String,
    pick_count: i64,
    /// 单位：分。
    total_price: i64,
    candidate_ids: Vec<i64>,
}

/// 已结算的展会账已冻结，不能再改定价。
///
/// ②-1 交接段列了 4 个「当前完全不查 `events.status`」的既有敞口留给 ②-3 的冻结语义。
/// **新增的写入路径不要再添第 5 个。**
async fn ensure_event_open(conn: &mut SqliteConnection, event_id: i64) -> ApiResult<()> {
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM events WHERE id = ?")
        .bind(event_id)
        .fetch_optional(&mut *conn)
        .await?;
    match status.as_deref() {
        None => Err(ApiError::NotFound("展会不存在".into())),
        Some("已结算") => Err(ApiError::Conflict("展会已结算，不能再改套装配置".into())),
        Some(_) => Ok(()),
    }
}

/// 校验候选集：非空、去重、全部属于本展会、**全部同一货主**。返回 (去重后的 id, 货主 id, 货主名)。
///
/// 同一货主是 spec 4.1 的硬约束：Lot 是摊主配的，代卖社团一样没参与这个决定，
/// 把他们的本子圈进「任选 3 本 100」等于替他们让价。将来真有两家事先谈好的联合套装，
/// 加一个 per-Lot 的「折让由谁承担」再放开，是纯增量。
async fn validate_candidates(
    conn: &mut SqliteConnection,
    event_id: i64,
    candidate_ids: &[i64],
) -> ApiResult<(Vec<i64>, i64, String)> {
    let mut ids: Vec<i64> = candidate_ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return Err(ApiError::BadRequest("套装至少要有一个候选商品".into()));
    }
    if ids.len() > MAX_CANDIDATES {
        return Err(ApiError::BadRequest("候选商品过多".into()));
    }

    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!(
        "SELECT ep.id, ep.owner_society_id, s.name
         FROM event_products ep
         JOIN societies s ON s.id = ep.owner_society_id
         WHERE ep.event_id = ? AND ep.id IN ({placeholders})"
    );
    let mut q = sqlx::query_as::<_, (i64, i64, String)>(sqlx::AssertSqlSafe(sql)).bind(event_id);
    for id in &ids {
        q = q.bind(*id);
    }
    let rows = q.fetch_all(&mut *conn).await?;

    if rows.len() != ids.len() {
        return Err(ApiError::BadRequest(
            "候选商品不存在或不属于本场展会".into(),
        ));
    }
    let owner = rows[0].1;
    if rows.iter().any(|(_, o, _)| *o != owner) {
        return Err(ApiError::BadRequest(
            "套装的候选商品必须属于同一个货主——替别的社团让价不是摊主能单方面决定的".into(),
        ));
    }
    let owner_name = rows[0].2.clone();
    Ok((ids, owner, owner_name))
}

fn validate_payload(payload: &LotPayload) -> ApiResult<String> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::BadRequest("套装名称不能为空".into()));
    }
    if payload.pick_count <= 0 {
        return Err(ApiError::BadRequest("「要选几件」必须是正数".into()));
    }
    if payload.total_price < 0 {
        return Err(ApiError::BadRequest("套装价格不能为负".into()));
    }
    // 候选集大小**不要求** >= pick_count：「任选 3 本 100」而候选只有一种书，
    // 意思是「买 3 本同款 100」，完全合法。
    Ok(name)
}

/// 把候选商品写进去。更新时先删后写，整组替换。
async fn write_candidates(conn: &mut SqliteConnection, lot_id: i64, ids: &[i64]) -> ApiResult<()> {
    sqlx::query("DELETE FROM lot_candidates WHERE lot_id = ?")
        .bind(lot_id)
        .execute(&mut *conn)
        .await?;
    for id in ids {
        sqlx::query("INSERT INTO lot_candidates (lot_id, event_product_id) VALUES (?, ?)")
            .bind(lot_id)
            .bind(*id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

/// `list_lots` 那条 JOIN 的行形状。抽成别名是为了过 `clippy::type_complexity`，
/// 语义与内联元组完全一样。
type LotListRow = (i64, i64, String, i64, i64, i64, i64, String);

async fn list_lots(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<Vec<LotResponse>>> {
    check_read_permission(&claims, event_id)?;

    // 一条 JOIN 查完再在 Rust 里分组，靠 ORDER BY l.id 保证同一个 Lot 的候选连续。
    let rows: Vec<LotListRow> = sqlx::query_as(
        "SELECT l.id, l.event_id, l.name, l.pick_count, l.total_price,
                lc.event_product_id, ep.owner_society_id, s.name
         FROM lots l
         JOIN lot_candidates lc ON lc.lot_id = l.id
         JOIN event_products ep ON ep.id = lc.event_product_id
         JOIN societies s ON s.id = ep.owner_society_id
         WHERE l.event_id = ?
         ORDER BY l.id, lc.event_product_id",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    let mut out: Vec<LotResponse> = Vec::new();
    for (id, ev, name, pick, price, candidate, owner, owner_name) in rows {
        match out.last_mut() {
            Some(last) if last.id == id => last.candidate_ids.push(candidate),
            _ => out.push(LotResponse {
                id,
                event_id: ev,
                name,
                pick_count: pick,
                total_price: price,
                candidate_ids: vec![candidate],
                owner_society_id: owner,
                owner_society_name: owner_name,
            }),
        }
    }
    Ok(Json(out))
}

async fn create_lot(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Json(payload): Json<LotPayload>,
) -> ApiResult<impl IntoResponse> {
    check_write_permission(&claims, event_id)?;
    let name = validate_payload(&payload)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    ensure_event_open(&mut tx, event_id).await?;
    let (ids, owner, owner_name) =
        validate_candidates(&mut tx, event_id, &payload.candidate_ids).await?;

    let lot_id: i64 = sqlx::query_scalar(
        "INSERT INTO lots (event_id, name, pick_count, total_price)
         VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(event_id)
    .bind(&name)
    .bind(payload.pick_count)
    .bind(payload.total_price)
    .fetch_one(&mut *tx)
    .await?;
    write_candidates(&mut tx, lot_id, &ids).await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(LotResponse {
            id: lot_id,
            event_id,
            name,
            pick_count: payload.pick_count,
            total_price: payload.total_price,
            candidate_ids: ids,
            owner_society_id: owner,
            owner_society_name: owner_name,
        }),
    ))
}

async fn update_lot(
    State(state): State<AppState>,
    claims: Claims,
    Path((event_id, lot_id)): Path<(i64, i64)>,
    Json(payload): Json<LotPayload>,
) -> ApiResult<Json<LotResponse>> {
    check_write_permission(&claims, event_id)?;
    let name = validate_payload(&payload)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    ensure_event_open(&mut tx, event_id).await?;

    let exists: Option<i64> =
        sqlx::query_scalar("SELECT id FROM lots WHERE id = ? AND event_id = ?")
            .bind(lot_id)
            .bind(event_id)
            .fetch_optional(&mut *tx)
            .await?;
    exists.ok_or_else(|| ApiError::NotFound("套装不存在".into()))?;

    let (ids, owner, owner_name) =
        validate_candidates(&mut tx, event_id, &payload.candidate_ids).await?;

    sqlx::query("UPDATE lots SET name = ?, pick_count = ?, total_price = ? WHERE id = ?")
        .bind(&name)
        .bind(payload.pick_count)
        .bind(payload.total_price)
        .bind(lot_id)
        .execute(&mut *tx)
        .await?;
    write_candidates(&mut tx, lot_id, &ids).await?;
    tx.commit().await?;

    Ok(Json(LotResponse {
        id: lot_id,
        event_id,
        name,
        pick_count: payload.pick_count,
        total_price: payload.total_price,
        candidate_ids: ids,
        owner_society_id: owner,
        owner_society_name: owner_name,
    }))
}

/// 删除永远放行。
///
/// `order_lots.lot_id` 是 `ON DELETE SET NULL`，而名字和价格在下单那一刻就
/// **快照**进了 `order_lots`——历史订单不受影响。所以这里不需要
/// 「被引用就不给删」那种守卫（`api/product.rs` 删商品时要，因为那边没有快照）。
async fn delete_lot(
    State(state): State<AppState>,
    claims: Claims,
    Path((event_id, lot_id)): Path<(i64, i64)>,
) -> ApiResult<impl IntoResponse> {
    check_write_permission(&claims, event_id)?;

    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    ensure_event_open(&mut tx, event_id).await?;
    let affected = sqlx::query("DELETE FROM lots WHERE id = ? AND event_id = ?")
        .bind(lot_id)
        .bind(event_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    tx.commit().await?;

    if affected == 0 {
        return Err(ApiError::NotFound("套装不存在".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, test_router_with,
    };
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn creating_a_lot_stores_its_candidates_and_reports_the_owner() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/lots"),
                Some(&token),
                json!({"name": "本子任选1本25", "pick_count": 1, "total_price": 2500,
                       "candidate_ids": [ep_a]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = read_json(res).await;
        assert_eq!(body["candidate_ids"], json!([ep_a]));
        assert_eq!(body["total_price"], 2500, "金额是分，不是元");
        assert_eq!(body["owner_society_id"], 1, "ep_a 归本社团");
    }

    #[tokio::test]
    async fn a_lot_spanning_two_owners_is_refused() {
        // spec 4.1：Lot 是摊主配的，代卖社团没参与这个决定。把他们的本子圈进
        // 「任选 3 本 100」等于替他们让价。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/lots"),
                Some(&token),
                json!({"name": "跨社团套装", "pick_count": 2, "total_price": 4000,
                       "candidate_ids": [ep_a, ep_b]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM lots")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "校验失败必须整体回滚，不能留下一个没有候选的 Lot");
    }

    #[tokio::test]
    async fn a_candidate_from_another_event_is_refused() {
        let (router, _dir, pool) = test_router_with().await;
        let (_event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query("INSERT INTO events (id, name, event_date, status) VALUES (2, '另一场', '2026-11-01', '进行中')")
            .execute(&pool).await.unwrap();
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/events/2/lots",
                Some(&token),
                json!({"name": "偷别场的商品", "pick_count": 1, "total_price": 100,
                       "candidate_ids": [ep_a]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn a_settled_event_refuses_new_lots() {
        // ②-1 交接段列了 4 个「当前完全不查 events.status」的既有敞口留给 ②-3。
        // 新增的写入路径不要再添第 5 个。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/lots"),
                Some(&token),
                json!({"name": "迟到的套装", "pick_count": 1, "total_price": 100,
                       "candidate_ids": [ep_a]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn updating_a_lot_replaces_its_whole_candidate_set() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        // event_products 有 UNIQUE(event_id, master_product_id)，所以先补一个全局商品。
        sqlx::query(
            "INSERT INTO master_products (id, product_code, name, default_price, owner_society_id)
             VALUES (3, 'C', '本子C', 25.0, 1)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO event_products (id, event_id, master_product_id, owner_society_id, product_code, name, unit_price)
             VALUES (3, 1, 3, 1, 'C', '本子C', 2500)",
        ).execute(&pool).await.unwrap();
        let token = admin_token();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/lots"), Some(&token),
            json!({"name": "旧", "pick_count": 1, "total_price": 2000, "candidate_ids": [ep_a]}),
        )).await.unwrap();
        let lot_id = read_json(res).await["id"].as_i64().unwrap();

        let res = router.clone().oneshot(json_request(
            "PUT", &format!("/api/events/{event_id}/lots/{lot_id}"), Some(&token),
            json!({"name": "新", "pick_count": 2, "total_price": 4500, "candidate_ids": [ep_a, 3]}),
        )).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        assert_eq!(body["name"], "新");
        assert_eq!(body["candidate_ids"], json!([ep_a, 3]));

        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM lot_candidates WHERE lot_id = ?")
            .bind(lot_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 2, "旧的候选必须被整组替换，不是追加");
    }

    #[tokio::test]
    async fn deleting_a_lot_leaves_past_orders_untouched() {
        // order_lots 存的是名字和价格的**快照**，lot_id 是 ON DELETE SET NULL。
        // 所以删 Lot 永远安全，不需要「被引用就不给删」的守卫。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router.clone().oneshot(json_request(
            "POST", &format!("/api/events/{event_id}/lots"), Some(&token),
            json!({"name": "会被删掉的套装", "pick_count": 1, "total_price": 2000, "candidate_ids": [ep_a]}),
        )).await.unwrap();
        let lot_id = read_json(res).await["id"].as_i64().unwrap();

        // 造一张引用了它的历史订单（Task 6 之前 create_order 还不会写 order_lots）
        sqlx::query("INSERT INTO orders (id, event_id, status, gross_amount, solved_amount, final_amount) VALUES (9, ?, 'pending', 3000, 2000, 2000)")
            .bind(event_id).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO order_lots (order_id, lot_id, name, price) VALUES (9, ?, '会被删掉的套装', 2000)")
            .bind(lot_id).execute(&pool).await.unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "DELETE",
                &format!("/api/events/{event_id}/lots/{lot_id}"),
                Some(&token),
                json!({}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        let (lot_ref, name, price): (Option<i64>, String, i64) =
            sqlx::query_as("SELECT lot_id, name, price FROM order_lots WHERE order_id = 9")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(lot_ref, None, "外键置空");
        assert_eq!(name, "会被删掉的套装", "名字快照还在");
        assert_eq!(price, 2000, "价格快照还在");
    }

    #[tokio::test]
    async fn listing_lots_needs_a_token() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/lots"),
                None,
                json!({}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}
