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
    routing::{get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;

use crate::{
    api::guard::{check_read_permission, check_write_permission},
    domain::pricing::{
        cart_lines, ensure_event_selling, load_lots, merge_items, price_cart, resolve_cart,
        CartItemRequest,
    },
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
        .route("/events/:event_id/quote", post(quote))
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
    // 上限 100 万元（分）。求解器在 `solver::best()` 里对套装价做 checked 加法，
    // 没有这条，一个 i64::MAX 的套装价会让那条加法溢出。
    if payload.total_price > 100_000_000 {
        return Err(ApiError::BadRequest("套装价格超出合理范围".into()));
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
///
/// 后三项是 `Option`：候选商品被删后 `lot_candidates` 的行被级联删掉，LEFT JOIN
/// 会给出 NULL。
type LotListRow = (
    i64,
    i64,
    String,
    i64,
    i64,
    Option<i64>,
    Option<i64>,
    Option<String>,
);

async fn list_lots(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> ApiResult<Json<Vec<LotResponse>>> {
    check_read_permission(&claims, event_id)?;

    // 一条 JOIN 查完再在 Rust 里分组，靠 ORDER BY l.id 保证同一个 Lot 的候选连续。
    //
    // 候选那几层必须是 LEFT JOIN：`lot_candidates.event_product_id` 是
    // `ON DELETE CASCADE`，而进货数为 0 的商品从来不会产生 stock_movements，
    // 所以可删。把某个 Lot 的最后一个候选删掉后，如果这里是 INNER JOIN，这个
    // Lot 会从管理页和求解器里同时消失——**界面上再也删不掉它**。
    // （求解器那条 `pricing::load_lots` 保持 INNER JOIN：空候选的 Lot 本来就
    // 套用不上，不该对求解器可见。）
    let rows: Vec<LotListRow> = sqlx::query_as(
        "SELECT l.id, l.event_id, l.name, l.pick_count, l.total_price,
                lc.event_product_id, ep.owner_society_id, s.name
         FROM lots l
         LEFT JOIN lot_candidates lc ON lc.lot_id = l.id
         LEFT JOIN event_products ep ON ep.id = lc.event_product_id
         LEFT JOIN societies s ON s.id = ep.owner_society_id
         WHERE l.event_id = ?
         ORDER BY l.id, lc.event_product_id",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    let mut out: Vec<LotResponse> = Vec::new();
    for (id, ev, name, pick, price, candidate, owner, owner_name) in rows {
        match out.last_mut() {
            Some(last) if last.id == id => {
                if let Some(candidate) = candidate {
                    last.candidate_ids.push(candidate);
                }
            }
            _ => out.push(LotResponse {
                id,
                event_id: ev,
                name,
                pick_count: pick,
                total_price: price,
                candidate_ids: candidate.into_iter().collect(),
                owner_society_id: owner.unwrap_or(0),
                owner_society_name: owner_name.unwrap_or_else(|| "（候选已被删除）".to_string()),
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

#[derive(Deserialize)]
struct QuoteRequest {
    items: Vec<CartItemRequest>,
}

#[derive(Serialize)]
struct QuoteMember {
    product_id: i64,
    qty: i64,
}

#[derive(Serialize)]
struct QuoteLot {
    lot_id: i64,
    name: String,
    /// 套装价（分）。
    price: i64,
    /// 成分按原价的合计（分）。
    ///
    /// 前端显示「已应用：XX −YY」时 `YY = original_amount − price`。放在后端算，
    /// 是因为前端再做一遍单价乘法就等于把分摊逻辑抄了半份出去。
    original_amount: i64,
    members: Vec<QuoteMember>,
}

#[derive(Serialize)]
struct QuoteLine {
    product_id: i64,
    qty: i64,
    /// 进了 `lots` 数组的第几个，`null` = 没进套装。
    lot_index: Option<usize>,
    allocated_amount: i64,
}

#[derive(Serialize)]
struct QuoteResponse {
    gross_amount: i64,
    solved_amount: i64,
    lots: Vec<QuoteLot>,
    lines: Vec<QuoteLine>,
}

/// 给购物车报价。**公开、无鉴权、零写入。**
///
/// 存在的唯一理由是 D3「顾客端价格必须可解释」：购物车要明写
/// 「已应用：本子任选3本100 −20.00」，而不是默默给个低价。
///
/// **不查库存**——报价是定价预览，库存由下单把关（所以这里也不必开事务）。
///
/// **报价永远不被信任**：下单时服务端用同一份 `price_cart` 独立重算，顾客最终付的
/// 金额取自**下单响应**而不是这里。两次之间摊主完全可能刚改过 Lot 配置。
async fn quote(
    State(state): State<AppState>,
    Path(event_id): Path<i64>,
    Json(payload): Json<QuoteRequest>,
) -> ApiResult<Json<QuoteResponse>> {
    let merged = merge_items(&payload.items)?;
    let mut conn = state.db.acquire().await?;
    ensure_event_selling(&mut conn, event_id).await?;
    let resolved = resolve_cart(&mut conn, event_id, &merged).await?;
    let lots = load_lots(&mut conn, event_id).await?;
    let priced = price_cart(&cart_lines(&resolved), &lots)?;

    // 成分原价合计：从 priced.lines 里按 lot_index 聚合，而不是重新乘一遍，
    // 保证和分摊用的是同一组数。
    let mut original: Vec<i64> = vec![0; priced.lots.len()];
    for line in &priced.lines {
        if let Some(k) = line.lot_index {
            // 这里是未检查乘法，但安全：price_cart 对**同一组** (unit_price, qty)
            // 已经跑过 checked_mul_qty，溢出会在那一步提前返回 Err，能走到这里就已经证明乘不爆。
            // 不要改成 checked——那会多一条永远走不到的错误分支；将来挪动 price_cart 里的检查位置时，
            // 必须同步确认这层依赖仍然成立。
            original[k] += line.unit_price.cents() * line.qty;
        }
    }

    Ok(Json(QuoteResponse {
        gross_amount: priced.gross.cents(),
        solved_amount: priced.solved.cents(),
        lots: priced
            .lots
            .iter()
            .enumerate()
            .map(|(k, l)| QuoteLot {
                lot_id: l.lot_id,
                name: l.name.clone(),
                price: l.price.cents(),
                original_amount: original[k],
                members: l
                    .members
                    .iter()
                    .map(|(id, q)| QuoteMember {
                        product_id: *id,
                        qty: *q,
                    })
                    .collect(),
            })
            .collect(),
        lines: priced
            .lines
            .iter()
            .map(|l| QuoteLine {
                product_id: l.event_product_id,
                qty: l.qty,
                lot_index: l.lot_index,
                allocated_amount: l.allocated.cents(),
            })
            .collect(),
    }))
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
    async fn a_lot_whose_candidates_were_all_deleted_stays_listed_and_deletable() {
        // `lot_candidates.event_product_id` 是 ON DELETE CASCADE，而 delete_product
        // 只在有 stock_movements 时才拒绝——进货数为 0 的商品从来不产生移动，所以可删。
        // 删光某个 Lot 的候选后，它必须仍然出现在 GET /lots 里，否则界面上再也删不掉它。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _ep_a, _ep_b) = seed_event_and_product(&pool).await;
        sqlx::query(
            "INSERT INTO master_products (id, product_code, name, default_price, owner_society_id)
             VALUES (3, 'C', '没进过货的本子', 25.0, 1)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO event_products (id, event_id, master_product_id, owner_society_id, product_code, name, unit_price)
             VALUES (3, 1, 3, 1, 'C', '没进过货的本子', 2500)",
        )
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
                json!({"name": "会被删空候选的套装", "pick_count": 1, "total_price": 2000,
                       "candidate_ids": [3]}),
            ))
            .await
            .unwrap();
        let lot_id = read_json(res).await["id"].as_i64().unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "DELETE",
                "/api/products/3",
                Some(&token),
                json!({}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "没有移动流水的商品可删");

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/lots"),
                Some(&token),
                json!({}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        let lots = body.as_array().unwrap();
        assert_eq!(lots.len(), 1, "候选被删光的 Lot 不能跟着从管理页消失");
        assert_eq!(lots[0]["id"], lot_id);
        assert_eq!(lots[0]["candidate_ids"], json!([]));
        assert_eq!(lots[0]["owner_society_id"], 0);
        assert_eq!(lots[0]["owner_society_name"], "（候选已被删除）");

        // 界面上唯一能删它的入口就是这一行，所以 DELETE 必须放行。
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

    /// 给 seed 出来的展会加一个只含 ep_a 的「任选 2 件 50」，返回 lot_id。
    async fn seed_pair_lot(router: &axum::Router, event_id: i64, ep_a: i64, token: &str) -> i64 {
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/lots"),
                Some(token),
                json!({"name": "任选2件50", "pick_count": 2, "total_price": 5000,
                       "candidate_ids": [ep_a]}),
            ))
            .await
            .unwrap();
        read_json(res).await["id"].as_i64().unwrap()
    }

    #[tokio::test]
    async fn quoting_without_any_lot_returns_the_gross_price() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/quote"),
                None, // 公开端点
                json!({"items": [{"product_id": ep_a, "quantity": 2},
                                 {"product_id": ep_b, "quantity": 1}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        assert_eq!(body["gross_amount"], 8000);
        assert_eq!(body["solved_amount"], 8000);
        assert_eq!(body["lots"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn quoting_applies_the_best_lot_and_says_how_much_it_saved() {
        // D3：顾客端价格必须可解释。购物车要能写出「已应用：任选2件50 −10.00」，
        // 所以响应里必须同时有套装价和成分原价合计。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let token = admin_token();
        let lot_id = seed_pair_lot(&router, event_id, ep_a, &token).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/quote"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 3}]}),
            ))
            .await
            .unwrap();
        let body = read_json(res).await;
        assert_eq!(body["gross_amount"], 9000, "3 × 30");
        assert_eq!(body["solved_amount"], 8000, "套装 50 + 散的 30");
        let lots = body["lots"].as_array().unwrap();
        assert_eq!(lots.len(), 1);
        assert_eq!(lots[0]["lot_id"], lot_id);
        assert_eq!(lots[0]["price"], 5000);
        assert_eq!(lots[0]["original_amount"], 6000, "省了 10 元");
        assert_eq!(lots[0]["members"], json!([{"product_id": ep_a, "qty": 2}]));

        // 同一个商品被拆成两行：2 件在套装里、1 件散着
        let lines = body["lines"].as_array().unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["lot_index"], 0);
        assert_eq!(lines[0]["allocated_amount"], 5000);
        assert!(lines[1]["lot_index"].is_null());
        assert_eq!(lines[1]["allocated_amount"], 3000);
    }

    #[tokio::test]
    async fn quoting_writes_nothing() {
        // 报价是只读的。留下订单或 journal 就是灾难——它是公开未鉴权端点。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journals")
            .fetch_one(&pool)
            .await
            .unwrap();

        let _ = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/quote"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 2}]}),
            ))
            .await
            .unwrap();

        let orders: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders")
            .fetch_one(&pool)
            .await
            .unwrap();
        let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM journals")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(orders, 0);
        assert_eq!(after, before);
    }

    #[tokio::test]
    async fn quoting_ignores_stock_because_it_is_only_a_price_preview() {
        // 现场仓只有 10 件，报 50 件的价照样出——库存由下单把关。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/quote"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 50}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(read_json(res).await["gross_amount"], 150_000);
    }

    #[tokio::test]
    async fn quoting_a_product_from_another_event_is_not_found() {
        let (router, _dir, pool) = test_router_with().await;
        let (_event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query("INSERT INTO events (id, name, event_date, status) VALUES (2, '另一场', '2026-11-01', '进行中')")
            .execute(&pool).await.unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/events/2/quote",
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn quoting_a_settled_event_is_refused() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/quote"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 1}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }
}
