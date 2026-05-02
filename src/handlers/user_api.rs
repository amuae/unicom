use actix_web::{get, post, web, HttpResponse};
use serde::Deserialize;

use crate::AppState;
use crate::services::notify;
use crate::handlers::query::{perform_query_analysis, query_user_by_token, QueryResponse};

#[derive(Deserialize)]
pub struct SaveConfigRequest {
    pub nickname: Option<String>,
    pub query_password: Option<String>,
    pub auth_type: Option<String>,
    pub appid: Option<String>,
    pub token_online: Option<String>,
    pub notify_enabled: Option<i32>,
    pub notify_type: Option<String>,
    pub notify_params: Option<serde_json::Value>,
    pub notify_threshold: Option<i32>,
    pub query_interval: Option<i32>,
    pub notify_title: Option<String>,
    pub notify_subtitle: Option<String>,
    pub notify_content: Option<String>,
}

// ─────────────────────────────────────────────
// GET /query/data/{token} — 查询页面数据（返回分析结果）
// ─────────────────────────────────────────────

#[get("/query/data/{token}")]
pub async fn query_page_data(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let token = path.into_inner();

    match perform_query_analysis(&state, &token).await {
        Ok(data) => HttpResponse::Ok().json(QueryResponse {
            success: true,
            data: Some(data),
            error: None,
        }),
        Err(e) => {
            let mut status = if e.contains("用户不存在") {
                HttpResponse::NotFound()
            } else {
                HttpResponse::InternalServerError()
            };
            status.json(QueryResponse {
                success: false,
                data: None,
                error: Some(e),
            })
        }
    }
}

// ─────────────────────────────────────────────
// POST /api/user/{token}/reset — 重置统计基准
// ─────────────────────────────────────────────

#[post("/user/{token}/reset")]
pub async fn reset_baseline(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let token = path.into_inner();
    let db = state.db.get().unwrap();

    let user_id: i64 = match db.query_row(
        "SELECT id FROM users WHERE token = ?1 AND status = 'active'",
        [&token],
        |row| row.get(0),
    ) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::NotFound().json(QueryResponse {
                success: false,
                data: None,
                error: Some("用户不存在或已被禁用".to_string()),
            });
        }
    };

    if let Err(e) = db.execute(
        "UPDATE users SET last_query_data = '', last_query_time = NULL WHERE id = ?1",
        [user_id],
    ) {
        return HttpResponse::InternalServerError().json(QueryResponse {
            success: false,
            data: None,
            error: Some(format!("重置失败: {}", e)),
        });
    }

    HttpResponse::Ok().json(QueryResponse {
        success: true,
        data: Some(serde_json::json!({"message": "统计基准已重置"})),
        error: None,
    })
}

// ─────────────────────────────────────────────
// GET /api/user/{token}/config — 获取用户配置
// ─────────────────────────────────────────────

#[get("/user/{token}/config")]
pub async fn get_user_config(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let token = path.into_inner();
    let db = state.db.get().unwrap();

    let result = db.query_row(
        "SELECT id, mobile, nickname, query_password, auth_type, appid, token_online, cookie, token, \
         notify_enabled, notify_type, notify_params, notify_threshold, query_interval, \
         notify_title, notify_subtitle, notify_content \
         FROM users WHERE token = ?1",
        [&token],
        |row| {
            Ok(serde_json::json!({
                "user_id": row.get::<_, i64>(0)?,
                "mobile": row.get::<_, String>(1)?,
                "nickname": row.get::<_, String>(2)?,
                "query_password": row.get::<_, String>(3)?,
                "auth_type": row.get::<_, String>(4)?,
                "appid": row.get::<_, String>(5)?,
                "token_online": row.get::<_, String>(6)?,
                "cookie": row.get::<_, String>(7)?,
                "token": row.get::<_, String>(8)?,
                "notify_enabled": row.get::<_, i32>(9)?,
                "notify_type": row.get::<_, String>(10)?,
                "notify_params": row.get::<_, String>(11)?,
                "notify_threshold": row.get::<_, i32>(12)?,
                "query_interval": row.get::<_, i32>(13)?,
                "notify_title": row.get::<_, String>(14)?,
                "notify_subtitle": row.get::<_, String>(15)?,
                "notify_content": row.get::<_, String>(16)?,
            }))
        },
    );

    match result {
        Ok(data) => HttpResponse::Ok().json(QueryResponse {
            success: true,
            data: Some(data),
            error: None,
        }),
        Err(_) => HttpResponse::NotFound().json(QueryResponse {
            success: false,
            data: None,
            error: Some("用户不存在".to_string()),
        }),
    }
}

// ─────────────────────────────────────────────
// POST /api/user/{token}/config — 保存用户配置
// ─────────────────────────────────────────────

#[post("/user/{token}/config")]
pub async fn save_user_config(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<SaveConfigRequest>,
) -> HttpResponse {
    let token = path.into_inner();
    let db = state.db.get().unwrap();

    let user_id: i64 = match db.query_row(
        "SELECT id FROM users WHERE token = ?1",
        [&token],
        |row| row.get(0),
    ) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::NotFound().json(QueryResponse {
                success: false,
                data: None,
                error: Some("用户不存在".to_string()),
            });
        }
    };

    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref nickname) = body.nickname {
        updates.push("nickname = ?");
        params.push(Box::new(nickname.clone()));
    }
    if let Some(ref query_password) = body.query_password {
        updates.push("query_password = ?");
        params.push(Box::new(query_password.clone()));
    }
    if let Some(ref auth_type) = body.auth_type {
        updates.push("auth_type = ?");
        params.push(Box::new(auth_type.clone()));
    }
    if let Some(ref appid) = body.appid {
        updates.push("appid = ?");
        params.push(Box::new(appid.clone()));
    }
    if let Some(ref token_online) = body.token_online {
        updates.push("token_online = ?");
        params.push(Box::new(token_online.clone()));
    }
    if let Some(enabled) = body.notify_enabled {
        updates.push("notify_enabled = ?");
        params.push(Box::new(enabled));
    }
    if let Some(ref notify_type) = body.notify_type {
        updates.push("notify_type = ?");
        params.push(Box::new(notify_type.clone()));
    }
    if let Some(ref notify_params) = body.notify_params {
        updates.push("notify_params = ?");
        params.push(Box::new(notify_params.to_string()));
    }
    if let Some(threshold) = body.notify_threshold {
        updates.push("notify_threshold = ?");
        params.push(Box::new(threshold));
    }
    if let Some(interval) = body.query_interval {
        updates.push("query_interval = ?");
        params.push(Box::new(interval));
    }
    if let Some(ref title) = body.notify_title {
        updates.push("notify_title = ?");
        params.push(Box::new(title.clone()));
    }
    if let Some(ref subtitle) = body.notify_subtitle {
        updates.push("notify_subtitle = ?");
        params.push(Box::new(subtitle.clone()));
    }
    if let Some(ref content) = body.notify_content {
        updates.push("notify_content = ?");
        params.push(Box::new(content.clone()));
    }

    if updates.is_empty() {
        return HttpResponse::BadRequest().json(QueryResponse {
            success: false,
            data: None,
            error: Some("没有要更新的字段".to_string()),
        });
    }

    updates.push("updated_at = datetime('now', 'localtime')");
    let sql = format!("UPDATE users SET {} WHERE id = ?", updates.join(", "));
    params.push(Box::new(user_id));

    if let Err(e) = db.execute(&sql, rusqlite::params_from_iter(params.iter())) {
        return HttpResponse::InternalServerError().json(QueryResponse {
            success: false,
            data: None,
            error: Some(format!("保存配置失败: {}", e)),
        });
    }

    // ── 检查是否需要创建/删除定时任务 ──
    // 查询用户当前配置
    let user_config: Option<(String, i32, i32)> = db.query_row(
        "SELECT COALESCE(notify_type, ''), COALESCE(notify_threshold, 0), COALESCE(query_interval, 5) \
         FROM users WHERE id = ?1",
        [user_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).ok();

    if let Some((notify_type, notify_threshold, query_interval)) = user_config {
        // 检查三项条件：通知方式正确、通知阈值已配置、查询间隔已配置
        let should_have_cron = !notify_type.is_empty() 
            && notify_type != "none"
            && notify_threshold > 0 
            && query_interval > 0;

        if should_have_cron {
            // 生成 cron 表达式（根据 query_interval）
            let cron_expr = if query_interval >= 60 {
                let hours = query_interval / 60;
                format!("0 0 */{} * * *", hours)
            } else {
                format!("0 */{} * * * *", query_interval)
            };

            // 获取用户手机号
            let mobile: String = db.query_row(
                "SELECT mobile FROM users WHERE id = ?1",
                [user_id],
                |row| row.get(0),
            ).unwrap_or_default();

            // 插入或更新定时任务
            let _ = db.execute(
                "INSERT INTO user_cron_tasks (user_id, mobile, cron_expression, status) \
                 VALUES (?1, ?2, ?3, 'active') \
                 ON CONFLICT(user_id) DO UPDATE SET \
                 cron_expression = ?3, status = 'active'",
                rusqlite::params![user_id, mobile, cron_expr],
            );

            tracing::info!("用户 {} 定时任务已创建/更新: {}", mobile, cron_expr);
        } else {
            // 条件不满足，删除该用户的定时任务
            let _ = db.execute(
                "DELETE FROM user_cron_tasks WHERE user_id = ?1",
                [user_id],
            );

            tracing::info!("用户 {} 配置不完整，定时任务已删除", user_id);
        }
    }

    HttpResponse::Ok().json(QueryResponse {
        success: true,
        data: Some(serde_json::json!({"message": "配置已保存"})),
        error: None,
    })
}

// ─────────────────────────────────────────────
// POST /api/user/{token}/test-notify — 测试通知
// ─────────────────────────────────────────────

#[post("/user/{token}/test-notify")]
pub async fn test_notify(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let token = path.into_inner();

    // 在锁外获取用户（避免持锁进行网络请求）
    let user = {
        let db = state.db.get().unwrap();
        query_user_by_token(&db, &token)
    };

    let user = match user {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::NotFound().json(QueryResponse {
                success: false,
                data: None,
                error: Some("用户不存在或已被禁用".to_string()),
            });
        }
    };

    match notify::send_test_notify(&user).await {
        Ok(()) => HttpResponse::Ok().json(QueryResponse {
            success: true,
            data: Some(serde_json::json!({"message": "测试通知已发送"})),
            error: None,
        }),
        Err(e) => HttpResponse::InternalServerError().json(QueryResponse {
            success: false,
            data: None,
            error: Some(format!("发送测试通知失败: {}", e)),
        }),
    }
}

// ─────────────────────────────────────────────
// GET /api/user/{token}/page — 返回查询页面 HTML
// ─────────────────────────────────────────────

#[get("/user/{token}/page")]
pub async fn serve_page(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let _token = path.into_inner();

    // SPA 应用：直接返回 index.html，前端根据 URL 中的 token 加载数据
    if let Some(file) = state.static_files.get("index.html") {
        HttpResponse::Ok()
            .insert_header(("Cache-Control", "no-cache, no-store, must-revalidate"))
            .content_type(file.content_type)
            .body(file.content)
    } else {
        HttpResponse::NotFound().body("index.html not found")
    }
}

// ─────────────────────────────────────────────
// DELETE /user/{token} — 用户自行删除账号
// ─────────────────────────────────────────────

#[actix_web::delete("/user/{token}")]
pub async fn delete_user_by_token(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let token = path.into_inner();
    let db = state.db.get().unwrap();

    let result = db.execute(
        "DELETE FROM users WHERE token = ?1",
        rusqlite::params![token],
    );

    match result {
        Ok(rows) if rows > 0 => {
            // 同时删除该用户的定时任务
            let _ = db.execute(
                "DELETE FROM user_cron_tasks WHERE user_id NOT IN (SELECT id FROM users)",
                [],
            );
            HttpResponse::Ok().json(QueryResponse {
                success: true,
                data: Some(serde_json::json!({"message": "账号已删除"})),
                error: None,
            })
        }
        Ok(_) => HttpResponse::NotFound().json(QueryResponse {
            success: false,
            data: None,
            error: Some("用户不存在".to_string()),
        }),
        Err(e) => HttpResponse::InternalServerError().json(QueryResponse {
            success: false,
            data: None,
            error: Some(format!("删除失败: {}", e)),
        }),
    }
}