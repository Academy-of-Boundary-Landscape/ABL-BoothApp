//! 社团 = 货主的单位。没有「个人」这一档（spec 3.1）。
//!
//! 「本社团」（`is_home`）有且只有一个，由 `idx_societies_home` 这个偏唯一索引
//! 在 DB 层保证。因此**升格一个新的本社团必须先把旧的降级**，顺序反了会撞唯一约束。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, put},
    Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    api::guard::AdminOnly,
    db::models::Society,
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_societies).post(create_society))
        .route("/:id", put(update_society).delete(delete_society))
}

#[derive(Deserialize)]
struct CreateSocietyRequest {
    name: String,
}

#[derive(Deserialize)]
struct UpdateSocietyRequest {
    name: Option<String>,
    is_home: Option<bool>,
}

/// 本社团排第一，其余按名字。选品下拉框里本社团永远在最上面。
async fn list_societies(State(state): State<AppState>, _: Claims) -> ApiResult<Json<Vec<Society>>> {
    let rows: Vec<Society> =
        sqlx::query_as("SELECT id, name, is_home FROM societies ORDER BY is_home DESC, name")
            .fetch_all(&state.db)
            .await?;
    Ok(Json(rows))
}

async fn create_society(
    State(state): State<AppState>,
    _: AdminOnly,
    Json(payload): Json<CreateSocietyRequest>,
) -> ApiResult<impl IntoResponse> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("社团名不能为空".into()));
    }

    let existing: Option<i64> = sqlx::query_scalar("SELECT id FROM societies WHERE name = ?")
        .bind(name)
        .fetch_optional(&state.db)
        .await?;
    if existing.is_some() {
        return Err(ApiError::Conflict(format!("社团「{name}」已存在")));
    }

    let row: Society = sqlx::query_as(
        "INSERT INTO societies (name, is_home) VALUES (?, 0) RETURNING id, name, is_home",
    )
    .bind(name)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(row)))
}

async fn update_society(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateSocietyRequest>,
) -> ApiResult<Json<Society>> {
    let mut tx = state.db.begin().await?;

    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM societies WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    if exists.is_none() {
        return Err(ApiError::NotFound("社团不存在".into()));
    }

    if let Some(name) = payload.name.as_deref() {
        let name = name.trim();
        if name.is_empty() {
            return Err(ApiError::BadRequest("社团名不能为空".into()));
        }
        sqlx::query("UPDATE societies SET name = ? WHERE id = ?")
            .bind(name)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }

    if payload.is_home == Some(true) {
        // 顺序不能反：偏唯一索引 idx_societies_home 会让「先升格新的」撞唯一约束。
        sqlx::query("UPDATE societies SET is_home = 0 WHERE is_home = 1")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE societies SET is_home = 1 WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    } else if payload.is_home == Some(false) {
        // 不允许把本社团降级成「没有本社团」——垫付、手工折让都要往它头上记。
        return Err(ApiError::BadRequest(
            "不能取消本社团标记，只能把另一个社团设为本社团".into(),
        ));
    }

    let row: Society = sqlx::query_as("SELECT id, name, is_home FROM societies WHERE id = ?")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(row))
}

async fn delete_society(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
) -> ApiResult<impl IntoResponse> {
    let is_home: Option<bool> = sqlx::query_scalar("SELECT is_home FROM societies WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;

    match is_home {
        None => return Err(ApiError::NotFound("社团不存在".into())),
        Some(true) => return Err(ApiError::Conflict("本社团不能删除".into())),
        Some(false) => {}
    }

    // 被引用就不给删。硬删会让历史账里的 owner_society_id 变成悬空外键，
    // 而 SQLite 的 FK 在这里是 NO ACTION，报出来的错对用户毫无意义。
    let refs: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM master_products WHERE owner_society_id = ?1)
              + (SELECT COUNT(*) FROM event_products  WHERE owner_society_id = ?1)",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    if refs > 0 {
        return Err(ApiError::Conflict(format!(
            "还有 {refs} 个商品归属这个社团，不能删除"
        )));
    }

    sqlx::query("DELETE FROM societies WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok((StatusCode::OK, Json(json!({"message": "社团已删除"}))))
}

#[cfg(test)]
mod tests {
    use crate::test_support::{admin_token, json_request, read_json, test_router};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use tower::ServiceExt; // for oneshot

    #[tokio::test]
    async fn fresh_database_has_exactly_one_home_society() {
        let (router, _dir) = test_router().await;
        let res = router
            .oneshot(
                Request::builder()
                    .uri("/api/societies")
                    .header("authorization", format!("Bearer {}", admin_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body = read_json(res).await;
        let arr = body.as_array().expect("数组");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "本社团");
        assert_eq!(arr[0]["is_home"], true);
    }

    #[tokio::test]
    async fn promoting_a_society_to_home_demotes_the_previous_one() {
        // 这条守的是 idx_societies_home 那个偏唯一索引不会被绕过：
        // 直接 UPDATE 新的为 is_home=1 会撞唯一约束，必须先降级旧的。
        let (router, _dir) = test_router().await;
        let token = admin_token();

        let res = router
            .clone() // oneshot 消费 router，多请求必须 clone
            .oneshot(json_request(
                "POST",
                "/api/societies",
                Some(&token),
                json!({"name": "黄昏堂"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let new_id = read_json(res).await["id"].as_i64().unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/societies/{new_id}"),
                Some(&token),
                json!({"is_home": true}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = router
            .oneshot(
                Request::builder()
                    .uri("/api/societies")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = read_json(res).await;
        let homes: Vec<_> = body
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["is_home"] == true)
            .collect();
        assert_eq!(homes.len(), 1, "本社团必须有且只有一个");
        assert_eq!(homes[0]["id"], new_id);
    }

    #[tokio::test]
    async fn home_society_cannot_be_deleted() {
        let (router, _dir) = test_router().await;
        let res = router
            .oneshot(json_request(
                "DELETE",
                "/api/societies/1",
                Some(&admin_token()),
                json!({}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn creating_a_society_requires_admin() {
        let (router, _dir) = test_router().await;
        let res = router
            .oneshot(json_request(
                "POST",
                "/api/societies",
                None,
                json!({"name": "X"}),
            ))
            .await
            .unwrap();
        // 无 token 走 Claims 提取器的 WrongCredentials → 401（不是 403，
        // 403 是「有 token 但不是 admin」）
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}
