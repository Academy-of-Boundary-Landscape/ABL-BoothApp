//! 社团 = 货主的单位。没有「个人」这一档（spec 3.1）。
//!
//! 「本社团」（`is_home`）有且只有一个，由 `idx_societies_home` 这个偏唯一索引
//! 在 DB 层保证。因此**升格一个新的本社团必须先把旧的降级**，顺序反了会撞唯一约束。

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{guard::AdminOnly, openapi::ApiErrorBody},
    db::models::Society,
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::Claims,
};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_societies, create_society))
        .routes(routes!(update_society, delete_society))
}

#[derive(Deserialize, ToSchema)]
struct CreateSocietyRequest {
    name: String,
}

#[derive(Deserialize, ToSchema)]
struct UpdateSocietyRequest {
    name: Option<String>,
    is_home: Option<bool>,
}

/// 删除社团的响应体。
#[derive(Serialize, ToSchema)]
struct DeleteSocietyResponse {
    message: String,
}

/// 社团列表：本社团排第一，其余按名字。选品下拉框里本社团永远在最上面。
#[utoipa::path(
    get,
    path = "/",
    tag = "society",
    security(("bearer" = [])),
    responses(
        (status = 200, body = Vec<Society>, description = "本社团在前，其余按名字"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
    ),
)]
async fn list_societies(State(state): State<AppState>, _: Claims) -> ApiResult<Json<Vec<Society>>> {
    let rows: Vec<Society> =
        sqlx::query_as("SELECT id, name, is_home FROM societies ORDER BY is_home DESC, name")
            .fetch_all(&state.db)
            .await?;
    Ok(Json(rows))
}

/// 新建一个社团。需要管理员。
#[utoipa::path(
    post,
    path = "/",
    tag = "society",
    request_body = CreateSocietyRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, body = Society),
        (status = 400, body = ApiErrorBody, description = "社团名不能为空"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 409, body = ApiErrorBody, description = "社团已存在"),
    ),
)]
async fn create_society(
    State(state): State<AppState>,
    _: AdminOnly,
    Json(payload): Json<CreateSocietyRequest>,
) -> ApiResult<(StatusCode, Json<Society>)> {
    // 不需要展会守卫：社团是全局实体，尚未挂任何展会的账
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

/// 修改社团名，或把它设为本社团（旧的会自动降级）。需要管理员。
#[utoipa::path(
    put,
    path = "/{id}",
    tag = "society",
    params(("id" = i64, Path, description = "社团 id")),
    request_body = UpdateSocietyRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, body = Society),
        (status = 400, body = ApiErrorBody, description = "社团名为空，或试图取消本社团标记"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 404, body = ApiErrorBody, description = "社团不存在"),
        (status = 409, body = ApiErrorBody, description = "社团名已存在"),
    ),
)]
async fn update_society(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateSocietyRequest>,
) -> ApiResult<Json<Society>> {
    // 不需要展会守卫：社团是全局实体，改名不动任何展会的账
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
        // 预先查重（排除自己），否则直接撞 name UNIQUE 约束会走 ApiError::Db → 500，
        // 与 create_society 的 409 语义不一致。改成自己现在的名字不该报冲突。
        let dup: Option<i64> =
            sqlx::query_scalar("SELECT id FROM societies WHERE name = ? AND id <> ?")
                .bind(name)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        if dup.is_some() {
            return Err(ApiError::Conflict(format!("社团「{name}」已存在")));
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

/// 删除一个社团。仍被商品 / 垫付 / 结算调整引用的不能删。需要管理员。
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "society",
    params(("id" = i64, Path, description = "社团 id")),
    security(("bearer" = [])),
    responses(
        (status = 200, body = DeleteSocietyResponse),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 404, body = ApiErrorBody, description = "社团不存在"),
        (status = 409, body = ApiErrorBody, description = "本社团，或仍被商品 / 垫付 / 结算调整引用"),
    ),
)]
async fn delete_society(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
) -> ApiResult<(StatusCode, Json<DeleteSocietyResponse>)> {
    // 不需要展会守卫：社团是全局实体，且被任何展会的货引用时本 handler 自己会拒绝
    let is_home: Option<bool> = sqlx::query_scalar("SELECT is_home FROM societies WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;

    match is_home {
        None => return Err(ApiError::NotFound("社团不存在".into())),
        Some(true) => return Err(ApiError::Conflict("本社团不能删除".into())),
        Some(false) => {}
    }

    // 被引用就不给删。四张表都带 owner_society_id，一张都不能漏——
    // 漏掉的那张会让守卫放行、然后在真正 DELETE 时撞 FK 约束，
    // 把一个本该是 409「还有东西挂着」的情况变成 500「数据库错误」。
    // advances / settlement_adjustments 在 ②-1 里恒为空（②-3 才写数据），
    // 但守卫现在就得覆盖它们，否则 ②-3 落地那天这个坑才会被踩到。
    let refs: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM master_products         WHERE owner_society_id = ?1)
              + (SELECT COUNT(*) FROM event_products          WHERE owner_society_id = ?1)
              + (SELECT COUNT(*) FROM advances                WHERE owner_society_id = ?1)
              + (SELECT COUNT(*) FROM settlement_adjustments  WHERE owner_society_id = ?1)",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    if refs > 0 {
        return Err(ApiError::Conflict(format!(
            "还有 {refs} 条记录（商品 / 垫付 / 结算调整）挂在这个社团上，不能删除"
        )));
    }

    sqlx::query("DELETE FROM societies WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok((
        StatusCode::OK,
        Json(DeleteSocietyResponse {
            message: "社团已删除".into(),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use crate::test_support::{
        admin_token, json_request, read_json, test_router, test_router_with,
    };
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

    #[tokio::test]
    async fn deleting_a_society_referenced_by_an_advance_is_refused_with_409() {
        // 守卫必须覆盖全部四张带 owner_society_id 的表。漏掉 advances /
        // settlement_adjustments 时，守卫会放行，然后真正 DELETE 撞 FK 约束 →
        // 500「数据库错误」，而不是干净的 409。
        let (router, _dir, pool) = test_router_with().await;

        sqlx::query("INSERT INTO societies (id, name, is_home) VALUES (2, '黄昏堂', 0)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO events (id, name, event_date, status)
             VALUES (1, 'E', '2026-10-01', '进行中')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO journals (id, event_id, kind) VALUES (1, 1, '垫付')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO advances (event_id, owner_society_id, journal_id, label, amount)
             VALUES (1, 2, 1, '摊位费', 40000)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let res = router
            .oneshot(json_request(
                "DELETE",
                "/api/societies/2",
                Some(&admin_token()),
                json!({}),
            ))
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            StatusCode::CONFLICT,
            "有垫付记录挂着时必须是 409，不能是 500"
        );
    }
}

/// ③b 形状快照：钉住每个路由的 JSON 形状（键 + 类型），类型化前后必须一行不改照样绿。
#[cfg(test)]
mod shape_tests {
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, shape_of, test_router_with,
        vendor_token,
    };
    use axum::http::StatusCode;
    use axum::Router;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    /// 迁移种下本社团（id=1, is_home=true）；夹具再插一个代卖社团「黄昏堂」（id=2,
    /// is_home=false）并让它的商品挂上去，好让 `is_home` 两种取值、以及「被引用不能删」
    /// 这条分支都有真实数据可打。
    async fn seeded() -> (Router, tempfile::TempDir, i64) {
        let (router, dir, pool) = test_router_with().await;
        let _ = seed_event_and_product(&pool).await;
        (router, dir, 2)
    }

    async fn call(
        router: &Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        body: Value,
    ) -> (StatusCode, Value) {
        let res = router
            .clone()
            .oneshot(json_request(method, uri, token, body))
            .await
            .unwrap();
        let status = res.status();
        (status, read_json(res).await)
    }

    fn society() -> Value {
        json!({"id": "int", "name": "string", "is_home": "bool"})
    }

    fn err() -> Value {
        json!({"error": "string"})
    }

    #[tokio::test]
    async fn shape_list_societies() {
        let (router, _dir, _) = seeded().await;
        let (s, body) = call(
            &router,
            "GET",
            "/api/societies",
            Some(&admin_token()),
            json!(null),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        // 本社团（is_home=true）与代卖社团（is_home=false）都在，数组形状合并。
        assert_eq!(shape_of(&body), json!([society()]));

        let (s, body) = call(&router, "GET", "/api/societies", None, json!(null)).await;
        assert_eq!((s, shape_of(&body)), (StatusCode::UNAUTHORIZED, err()));
    }

    #[tokio::test]
    async fn shape_create_society() {
        let (router, _dir, _) = seeded().await;
        let t = admin_token();

        let (s, body) = call(
            &router,
            "POST",
            "/api/societies",
            Some(&t),
            json!({"name": "红魔馆"}),
        )
        .await;
        assert_eq!((s, shape_of(&body)), (StatusCode::CREATED, society()));

        // 空名 → 400
        let (s, body) = call(
            &router,
            "POST",
            "/api/societies",
            Some(&t),
            json!({"name": "   "}),
        )
        .await;
        assert_eq!((s, shape_of(&body)), (StatusCode::BAD_REQUEST, err()));

        // 重名 → 409
        let (s, body) = call(
            &router,
            "POST",
            "/api/societies",
            Some(&t),
            json!({"name": "红魔馆"}),
        )
        .await;
        assert_eq!((s, shape_of(&body)), (StatusCode::CONFLICT, err()));

        // 非管理员 → 403
        let (s, body) = call(
            &router,
            "POST",
            "/api/societies",
            Some(&vendor_token(1)),
            json!({"name": "x"}),
        )
        .await;
        assert_eq!((s, shape_of(&body)), (StatusCode::FORBIDDEN, err()));
    }

    #[tokio::test]
    async fn shape_update_society() {
        let (router, _dir, sid) = seeded().await;
        let t = admin_token();

        let (s, body) = call(
            &router,
            "PUT",
            &format!("/api/societies/{sid}"),
            Some(&t),
            json!({"name": "黄昏堂改", "is_home": true}),
        )
        .await;
        assert_eq!((s, shape_of(&body)), (StatusCode::OK, society()));

        // 不存在 → 404
        let (s, body) = call(
            &router,
            "PUT",
            "/api/societies/9999",
            Some(&t),
            json!({"name": "x"}),
        )
        .await;
        assert_eq!((s, shape_of(&body)), (StatusCode::NOT_FOUND, err()));

        // 试图取消本社团标记 → 400
        let (s, body) = call(
            &router,
            "PUT",
            &format!("/api/societies/{sid}"),
            Some(&t),
            json!({"is_home": false}),
        )
        .await;
        assert_eq!((s, shape_of(&body)), (StatusCode::BAD_REQUEST, err()));
    }

    #[tokio::test]
    async fn shape_delete_society() {
        let (router, _dir, _) = seeded().await;
        let t = admin_token();

        // 新建一个没人引用的社团，删掉 → 200 {"message": ...}
        let (_, body) = call(
            &router,
            "POST",
            "/api/societies",
            Some(&t),
            json!({"name": "红魔馆"}),
        )
        .await;
        let id = body["id"].as_i64().unwrap();
        let (s, body) = call(
            &router,
            "DELETE",
            &format!("/api/societies/{id}"),
            Some(&t),
            json!(null),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::OK, json!({"message": "string"}))
        );

        // 本社团不能删 → 409
        let (s, body) = call(&router, "DELETE", "/api/societies/1", Some(&t), json!(null)).await;
        assert_eq!((s, shape_of(&body)), (StatusCode::CONFLICT, err()));

        // 不存在 → 404
        let (s, body) = call(
            &router,
            "DELETE",
            "/api/societies/9999",
            Some(&t),
            json!(null),
        )
        .await;
        assert_eq!((s, shape_of(&body)), (StatusCode::NOT_FOUND, err()));

        // 被商品引用 → 409
        let (s, body) = call(&router, "DELETE", "/api/societies/2", Some(&t), json!(null)).await;
        assert_eq!((s, shape_of(&body)), (StatusCode::CONFLICT, err()));
    }
}
