use actix_web::{get, put, web, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Serialize)]
pub struct UserConfigResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateConfigRequest {
    pub nickname: Option<String>,
    pub query_password: Option<String>,
    pub cookie: Option<String>,
    pub notify_enabled: Option<i32>,
    pub notify_type: Option<String>,
    pub notify_params: Option<serde_json::Value>,
    pub notify_threshold: Option<i32>,
    pub query_interval: Option<i32>,
}

#[get("/user/config/{token}")]
pub async fn get_user_config(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let token = path.into_inner();
    let db = match state.db.get() {
        Ok(db) => db,
        Err(_) => return HttpResponse::InternalServerError().json(UserConfigResponse {
            success: false, data: None, error: Some("数据库连接失败".to_string()),
        }),
    };

    let result = db.query_row(
        "SELECT id, mobile, nickname, auth_type, appid, token_online, cookie, token, \
         notify_enabled, notify_type, notify_params, notify_threshold, query_interval \
         FROM users WHERE token = ?1",
        [&token],
        |row| {
            Ok(serde_json::json!({
                "user_id": row.get::<_, i64>(0)?,
                "mobile": row.get::<_, String>(1)?,
                "nickname": row.get::<_, String>(2)?,
                "auth_type": row.get::<_, String>(3)?,
                "appid": row.get::<_, String>(4)?,
                "token_online": row.get::<_, String>(5)?,
                "cookie": row.get::<_, String>(6)?,
                "token": row.get::<_, String>(7)?,
                "notify_enabled": row.get::<_, i32>(8)?,
                "notify_type": row.get::<_, String>(9)?,
                "notify_params": serde_json::from_str::<serde_json::Value>(
                    &row.get::<_, String>(10)?
                ).unwrap_or(serde_json::Value::Null),
                "notify_threshold": row.get::<_, i32>(11)?,
                "query_interval": row.get::<_, i32>(12)?,
            }))
        },
    );

    match result {
        Ok(data) => HttpResponse::Ok().json(UserConfigResponse {
            success: true, data: Some(data), error: None,
        }),
        Err(_) => HttpResponse::NotFound().json(UserConfigResponse {
            success: false, data: None, error: Some("用户不存在".to_string()),
        }),
    }
}

#[put("/user/config/{token}")]
pub async fn update_user_config(
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<UpdateConfigRequest>,
) -> HttpResponse {
    let token = path.into_inner();
    let db = match state.db.get() {
        Ok(db) => db,
        Err(_) => return HttpResponse::InternalServerError().json(UserConfigResponse {
            success: false, data: None, error: Some("数据库连接失败".to_string()),
        }),
    };

    // 验证用户存在
    let user_id: i64 = match db.query_row(
        "SELECT id FROM users WHERE token = ?1",
        [&token],
        |row| row.get(0),
    ) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::NotFound().json(UserConfigResponse {
                success: false, data: None, error: Some("用户不存在".to_string()),
            });
        }
    };

    // 更新用户配置
    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(nickname) = &body.nickname {
        updates.push("nickname = ?");
        params.push(Box::new(nickname.clone()));
    }
    if let Some(password) = &body.query_password {
        updates.push("query_password = ?");
        params.push(Box::new(password.clone()));
    }
    if let Some(cookie) = &body.cookie {
        updates.push("cookie = ?");
        params.push(Box::new(cookie.clone()));
        updates.push("cookie_created_at = datetime('now', 'localtime')");
    }
    if let Some(enabled) = body.notify_enabled {
        updates.push("notify_enabled = ?");
        params.push(Box::new(enabled));
    }
    if let Some(notify_type) = &body.notify_type {
        updates.push("notify_type = ?");
        params.push(Box::new(notify_type.clone()));
    }
    if let Some(notify_params) = &body.notify_params {
        // 兼容前端发来 JSON 对象或 JSON 字符串，避免双重转义
        let params_str = match notify_params {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        updates.push("notify_params = ?");
        params.push(Box::new(params_str));
    }
    if let Some(threshold) = body.notify_threshold {
        updates.push("notify_threshold = ?");
        params.push(Box::new(threshold));
    }
    if let Some(interval) = body.query_interval {
        updates.push("query_interval = ?");
        params.push(Box::new(interval));
    }

    if updates.is_empty() {
        return HttpResponse::BadRequest().json(UserConfigResponse {
            success: false, data: None, error: Some("没有要更新的字段".to_string()),
        });
    }

    updates.push("updated_at = datetime('now', 'localtime')");
    let sql = format!("UPDATE users SET {} WHERE id = ?", updates.join(", "));
    params.push(Box::new(user_id));

    match db.execute(&sql, rusqlite::params_from_iter(params.iter())) {
        Ok(_) => HttpResponse::Ok().json(UserConfigResponse {
            success: true,
            data: Some(serde_json::json!({"message": "配置已更新"})),
            error: None,
        }),
        Err(e) => HttpResponse::InternalServerError().json(UserConfigResponse {
            success: false, data: None, error: Some(format!("更新失败: {}", e)),
        }),
    }
}
