//! 结算侧：渠道列表、垫付、结算调整、结算单、收摊清点、xlsx 导出。
//!
//! **本模块里有三个端点故意不调用 `require_event_open`**：垫付、结算调整、
//! 收摊清点。冻结之后它们仍然允许（spec 偏离 3）——「回家翻出一张打印费收据」
//! 和「回家发现少了一本书」是同一类事件。看见别处都守着就顺手补上去，
//! 会把有意的例外当成漏掉的守卫。改之前先读 spec 3.2。

use axum::{extract::State, routing::get, Json, Router};

use crate::{
    domain::channel::PRESET_CHANNELS, error::ApiResult, state::AppState, utils::security::Claims,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/channels", get(list_channels))
}

/// 已用过的收款渠道，跨展会。
///
/// 挂在 `/api/channels` 而不是 `/api/events/:id/channels`：它按定义就是跨展会的。
/// 这是防「微信」和「微信支付」分裂成两个账户的那一条（②-1/②-2 交接段第 3 条）。
async fn list_channels(
    State(state): State<AppState>,
    _claims: Claims,
) -> ApiResult<Json<Vec<String>>> {
    // 不需要展会守卫：只读，且按定义跨展会
    let used: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT channel FROM orders  WHERE channel IS NOT NULL AND channel <> ''
         UNION
         SELECT DISTINCT channel FROM refunds WHERE channel <> ''
         ORDER BY 1",
    )
    .fetch_all(&state.db)
    .await?;

    let mut out: Vec<String> = PRESET_CHANNELS.iter().map(|s| s.to_string()).collect();
    for c in used {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    Ok(Json(out))
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
    async fn channels_list_starts_with_the_three_presets() {
        let (router, _dir, pool) = test_router_with().await;
        seed_event_and_product(&pool).await;
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                "/api/channels",
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        let list: Vec<String> = serde_json::from_value(body).unwrap();
        assert_eq!(list, vec!["现金", "微信", "支付宝"]);
    }

    #[tokio::test]
    async fn channels_list_includes_history_across_events() {
        // 跨展会才有意义：上一场用过「银行转账」，这一场当然还想用。
        let (router, _dir, pool) = test_router_with().await;
        seed_event_and_product(&pool).await;
        sqlx::query(
            "INSERT INTO orders (event_id, status, channel, gross_amount, solved_amount, final_amount)
             VALUES (1, 'completed', '银行转账', 100, 100, 100)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let token = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                "/api/channels",
                Some(&token),
                json!(null),
            ))
            .await
            .unwrap();
        let list: Vec<String> = serde_json::from_value(read_json(res).await).unwrap();
        assert!(list.contains(&"银行转账".to_string()));
        assert_eq!(
            list.iter().filter(|c| *c == "现金").count(),
            1,
            "预置的不能重复出现"
        );
    }
}
