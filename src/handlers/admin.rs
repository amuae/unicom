use actix_web::{get, post, put, delete, web, HttpResponse};
use serde::{Deserialize, Serialize};
use bcrypt::{hash, verify, DEFAULT_COST};

use crate::AppState;

#[derive(Serialize)]
pub struct AdminResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
}

#[derive(Deserialize)]
pub struct AddUserRequest {
    pub mobile: String,
    pub nickname: Option<String>,
    pub query_password: String,
    pub auth_type: String,
    pub cookie: Option<String>,
    pub appid: Option<String>,
    pub token_online: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub id: i64,
    pub mobile: Option<String>,
    pub nickname: Option<String>,
    pub query_password: Option<String>,
    pub auth_type: Option<String>,
    pub cookie: Option<String>,
    pub appid: Option<String>,
    pub token_online: Option<String>,
    pub status: Option<String>,
    pub notify_enabled: Option<i32>,
    pub notify_type: Option<String>,
    pub notify_params: Option<String>,
    pub notify_threshold: Option<i32>,
    pub query_interval: Option<i32>,
    pub notify_title: Option<String>,
    pub notify_content: Option<String>,
}

#[derive(Deserialize)]
pub struct UserIdRequest {
    pub id: i64,
}

// ── Cron request types ──

#[derive(Deserialize)]
pub struct CronCreateRequest {
    pub user_id: i64,
    pub cron_expression: String,
}

#[derive(Deserialize)]
pub struct CronToggleRequest {
    pub id: i64,
}

#[derive(Deserialize)]
pub struct CronDeleteRequest {
    pub id: i64,
}

// ── System request types ──

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Deserialize)]
pub struct ChangeUsernameRequest {
    pub new_username: String,
}

#[derive(Deserialize)]
pub struct GuestRegisterRequest {
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub enabled: String,
}

#[derive(Deserialize)]
pub struct LogConfigRequest {
    pub log_level: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub log_retention_days: String,
}

fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;
    struct StringOrNumberVisitor;

    impl<'de> de::Visitor<'de> for StringOrNumberVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string or number")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<String, E> {
            Ok(v.to_string())
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<String, E> {
            Ok(v.to_string())
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<String, E> {
            Ok(v.to_string())
        }

        fn visit_f64<E: de::Error>(self, v: f64) -> Result<String, E> {
            Ok(v.to_string())
        }
    }

    deserializer.deserialize_any(StringOrNumberVisitor)
}

// ════════════════════════════════════════════════════════════════
// 用户管理
// ════════════════════════════════════════════════════════════════

#[get("/admin/users")]
pub async fn get_users(
    state: web::Data<AppState>,
) -> HttpResponse {
    let db = state.db.get().unwrap();

    let mut stmt = db.prepare(
        "SELECT id, mobile, nickname, auth_type, status, token, notify_enabled, notify_type, \
         query_interval, last_query_at, created_at \
         FROM users ORDER BY created_at DESC"
    ).unwrap();

    let users: Vec<serde_json::Value> = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "mobile": row.get::<_, String>(1)?,
            "nickname": row.get::<_, String>(2)?,
            "auth_type": row.get::<_, String>(3)?,
            "status": row.get::<_, String>(4)?,
            "token": row.get::<_, String>(5)?,
            "notify_enabled": row.get::<_, i32>(6)?,
            "notify_type": row.get::<_, String>(7)?,
            "query_interval": row.get::<_, i32>(8)?,
            "last_query_at": row.get::<_, Option<String>>(9)?,
            "created_at": row.get::<_, String>(10)?,
        }))
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect();

    HttpResponse::Ok().json(AdminResponse {
        success: true,
        data: Some(serde_json::json!(users)),
        error: None,
        total: None,
    })
}

#[get("/admin/users/{id}")]
pub async fn get_user_detail(
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> HttpResponse {
    let user_id = path.into_inner();
    let db = state.db.get().unwrap();

    let result = db.query_row(
        "SELECT id, mobile, nickname, query_password, auth_type, appid, token_online, \
         cookie, cookie_created_at, status, last_query_at, created_at, updated_at, \
         COALESCE(today_query_data, ''), COALESCE(last_query_data, ''), last_query_time, token, \
         notify_enabled, notify_type, notify_params, notify_title, notify_subtitle, \
         notify_content, notify_threshold, query_interval, last_notify_time \
         FROM users WHERE id = ?1",
        rusqlite::params![user_id],
        |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "mobile": row.get::<_, String>(1)?,
                "nickname": row.get::<_, String>(2)?,
                "query_password": row.get::<_, String>(3)?,
                "auth_type": row.get::<_, String>(4)?,
                "appid": row.get::<_, String>(5)?,
                "token_online": row.get::<_, String>(6)?,
                "cookie": row.get::<_, String>(7)?,
                "cookie_created_at": row.get::<_, Option<String>>(8)?,
                "status": row.get::<_, String>(9)?,
                "last_query_at": row.get::<_, Option<String>>(10)?,
                "created_at": row.get::<_, String>(11)?,
                "updated_at": row.get::<_, Option<String>>(12)?,
                "today_query_data": row.get::<_, String>(13)?,
                "last_query_data": row.get::<_, String>(14)?,
                "last_query_time": row.get::<_, Option<String>>(15)?,
                "token": row.get::<_, String>(16)?,
                "notify_enabled": row.get::<_, i32>(17)?,
                "notify_type": row.get::<_, String>(18)?,
                "notify_params": serde_json::from_str::<serde_json::Value>(
                    &row.get::<_, String>(19)?
                ).unwrap_or(serde_json::Value::Null),
                "notify_title": row.get::<_, String>(20)?,
                "notify_subtitle": row.get::<_, String>(21)?,
                "notify_content": row.get::<_, String>(22)?,
                "notify_threshold": row.get::<_, i32>(23)?,
                "query_interval": row.get::<_, i32>(24)?,
                "last_notify_time": row.get::<_, Option<String>>(25)?,
            }))
        },
    );

    match result {
        Ok(user) => HttpResponse::Ok().json(AdminResponse {
            success: true,
            data: Some(user),
            error: None,
            total: None,
        }),
        Err(_) => HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some("用户不存在".to_string()),
            total: None,
        }),
    }
}

#[post("/admin/users")]
pub async fn add_user(
    state: web::Data<AppState>,
    body: web::Json<AddUserRequest>,
) -> HttpResponse {
    let db = state.db.get().unwrap();
    
    // 检查手机号是否已存在
    let exists: bool = db.query_row(
        "SELECT COUNT(*) FROM users WHERE mobile = ?1",
        rusqlite::params![body.mobile],
        |row| row.get::<_, i64>(0),
    ).unwrap_or(0) > 0;
    
    if exists {
        return HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some("手机号已存在".to_string()),
            total: None,
        });
    }
    
    // 生成 token
    let token = uuid::Uuid::new_v4().to_string();
    
    let result = db.execute(
        "INSERT INTO users (mobile, nickname, query_password, auth_type, cookie, appid, token_online, token, status, \
         today_query_data, last_query_data) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active', '', '')",
        rusqlite::params![
            body.mobile,
            body.nickname.as_deref().unwrap_or(""),
            body.query_password,
            body.auth_type,
            body.cookie.as_deref().unwrap_or(""),
            body.appid.as_deref().unwrap_or(""),
            body.token_online.as_deref().unwrap_or(""),
            token,
        ],
    );
    
    match result {
        Ok(_) => HttpResponse::Ok().json(AdminResponse {
            success: true,
            data: Some(serde_json::json!({"message": "添加成功"})),
            error: None,
            total: None,
        }),
        Err(e) => HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some(format!("添加失败: {}", e)),
            total: None,
        }),
    }
}

#[put("/admin/users")]
pub async fn update_user(
    state: web::Data<AppState>,
    body: web::Json<UpdateUserRequest>,
) -> HttpResponse {
    let db = state.db.get().unwrap();
    
    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut param_idx = 1;
    
    if let Some(ref mobile) = body.mobile {
        updates.push(format!("mobile = ?{}", param_idx));
        params.push(Box::new(mobile.clone()));
        param_idx += 1;
    }
    if let Some(ref nickname) = body.nickname {
        updates.push(format!("nickname = ?{}", param_idx));
        params.push(Box::new(nickname.clone()));
        param_idx += 1;
    }
    if let Some(ref query_password) = body.query_password {
        updates.push(format!("query_password = ?{}", param_idx));
        params.push(Box::new(query_password.clone()));
        param_idx += 1;
    }
    if let Some(ref auth_type) = body.auth_type {
        updates.push(format!("auth_type = ?{}", param_idx));
        params.push(Box::new(auth_type.clone()));
        param_idx += 1;
    }
    if let Some(ref cookie) = body.cookie {
        updates.push(format!("cookie = ?{}", param_idx));
        params.push(Box::new(cookie.clone()));
        param_idx += 1;
    }
    if let Some(ref appid) = body.appid {
        updates.push(format!("appid = ?{}", param_idx));
        params.push(Box::new(appid.clone()));
        param_idx += 1;
    }
    if let Some(ref token_online) = body.token_online {
        updates.push(format!("token_online = ?{}", param_idx));
        params.push(Box::new(token_online.clone()));
        param_idx += 1;
    }
    if let Some(ref status) = body.status {
        updates.push(format!("status = ?{}", param_idx));
        params.push(Box::new(status.clone()));
        param_idx += 1;
    }
    if let Some(notify_enabled) = body.notify_enabled {
        updates.push(format!("notify_enabled = ?{}", param_idx));
        params.push(Box::new(notify_enabled));
        param_idx += 1;
    }
    if let Some(ref notify_type) = body.notify_type {
        updates.push(format!("notify_type = ?{}", param_idx));
        params.push(Box::new(notify_type.clone()));
        param_idx += 1;
    }
    if let Some(ref notify_params) = body.notify_params {
        updates.push(format!("notify_params = ?{}", param_idx));
        params.push(Box::new(notify_params.clone()));
        param_idx += 1;
    }
    if let Some(notify_threshold) = body.notify_threshold {
        updates.push(format!("notify_threshold = ?{}", param_idx));
        params.push(Box::new(notify_threshold));
        param_idx += 1;
    }
    if let Some(query_interval) = body.query_interval {
        updates.push(format!("query_interval = ?{}", param_idx));
        params.push(Box::new(query_interval));
        param_idx += 1;
    }
    if let Some(ref notify_title) = body.notify_title {
        updates.push(format!("notify_title = ?{}", param_idx));
        params.push(Box::new(notify_title.clone()));
        param_idx += 1;
    }
    if let Some(ref notify_content) = body.notify_content {
        updates.push(format!("notify_content = ?{}", param_idx));
        params.push(Box::new(notify_content.clone()));
        param_idx += 1;
    }
    
    if updates.is_empty() {
        return HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some("没有要更新的字段".to_string()),
            total: None,
        });
    }
    
    updates.push(format!("updated_at = datetime('now')"));
    
    let sql = format!(
        "UPDATE users SET {} WHERE id = ?{}",
        updates.join(", "),
        param_idx
    );
    params.push(Box::new(body.id));
    
    let result = db.execute(&sql, rusqlite::params_from_iter(params.iter()));
    
    match result {
        Ok(rows) => {
            if rows > 0 {
                // 写日志
                log_to_db(&db, "system", "info",
                    &format!("管理员更新了用户 #{}", body.id),
                    None, None, None);
                HttpResponse::Ok().json(AdminResponse {
                    success: true,
                    data: Some(serde_json::json!({"message": "更新成功"})),
                    error: None,
                    total: None,
                })
            } else {
                HttpResponse::Ok().json(AdminResponse {
                    success: false,
                    data: None,
                    error: Some("用户不存在".to_string()),
                    total: None,
                })
            }
        }
        Err(e) => HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some(format!("更新失败: {}", e)),
            total: None,
        }),
    }
}

#[delete("/admin/users")]
pub async fn delete_user(
    state: web::Data<AppState>,
    body: web::Json<UserIdRequest>,
) -> HttpResponse {
    let db = state.db.get().unwrap();
    
    let result = db.execute(
        "DELETE FROM users WHERE id = ?1",
        rusqlite::params![body.id],
    );
    
    match result {
        Ok(rows) => {
            if rows > 0 {
                log_to_db(&db, "system", "info",
                    &format!("管理员删除了用户 #{}", body.id),
                    None, None, None);
                HttpResponse::Ok().json(AdminResponse {
                    success: true,
                    data: Some(serde_json::json!({"message": "删除成功"})),
                    error: None,
                    total: None,
                })
            } else {
                HttpResponse::Ok().json(AdminResponse {
                    success: false,
                    data: None,
                    error: Some("用户不存在".to_string()),
                    total: None,
                })
            }
        }
        Err(e) => HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some(format!("删除失败: {}", e)),
            total: None,
        }),
    }
}

#[post("/admin/users/toggle")]
pub async fn toggle_user_status(
    state: web::Data<AppState>,
    body: web::Json<UserIdRequest>,
) -> HttpResponse {
    let db = state.db.get().unwrap();
    
    // 先获取当前状态
    let current_status: String = db.query_row(
        "SELECT status FROM users WHERE id = ?1",
        rusqlite::params![body.id],
        |row| row.get(0),
    ).unwrap_or_default();
    
    let new_status = if current_status == "active" { "disabled" } else { "active" };
    
    let result = db.execute(
        "UPDATE users SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![new_status, body.id],
    );
    
    match result {
        Ok(rows) => {
            if rows > 0 {
                log_to_db(&db, "system", "info",
                    &format!("管理员{}了用户 #{}", if new_status == "active" { "启用" } else { "禁用" }, body.id),
                    None, None, None);
                HttpResponse::Ok().json(AdminResponse {
                    success: true,
                    data: Some(serde_json::json!({
                        "message": if new_status == "active" { "已启用" } else { "已禁用" },
                        "new_status": new_status,
                    })),
                    error: None,
                    total: None,
                })
            } else {
                HttpResponse::Ok().json(AdminResponse {
                    success: false,
                    data: None,
                    error: Some("用户不存在".to_string()),
                    total: None,
                })
            }
        }
        Err(e) => HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some(format!("操作失败: {}", e)),
            total: None,
        }),
    }
}

// ════════════════════════════════════════════════════════════════
// 日志管理
// ════════════════════════════════════════════════════════════════

#[get("/admin/logs")]
pub async fn get_logs(
    state: web::Data<AppState>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let db = state.db.get().unwrap();
    let limit = query.get("limit").and_then(|l| l.parse::<i64>().ok()).unwrap_or(100);
    let offset = query.get("offset").and_then(|o| o.parse::<i64>().ok()).unwrap_or(0);
    let log_type = query.get("log_type").or_else(|| query.get("type")).map(|s| s.as_str());
    let level = query.get("level").map(|s| s.as_str());

    // Build dynamic WHERE clause
    let mut conditions = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    if let Some(t) = log_type {
        conditions.push(format!("log_type = ?{}", idx));
        params.push(Box::new(t.to_string()));
        idx += 1;
    }
    if let Some(l) = level {
        conditions.push(format!("log_level = ?{}", idx));
        params.push(Box::new(l.to_string()));
        idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT id, log_type, log_level, message, context, user_id, ip_address, created_at \
         FROM system_logs {} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
        where_clause, idx, idx + 1
    );
    params.push(Box::new(limit));
    params.push(Box::new(offset));

    let mut stmt = db.prepare(&sql).unwrap();

    let logs: Vec<serde_json::Value> = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "log_type": row.get::<_, String>(1)?,
            "log_level": row.get::<_, String>(2)?,
            "message": row.get::<_, String>(3)?,
            "context": row.get::<_, Option<String>>(4)?,
            "user_id": row.get::<_, Option<i64>>(5)?,
            "ip_address": row.get::<_, Option<String>>(6)?,
            "created_at": row.get::<_, String>(7)?,
        }))
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect();

    // Get total count
    let total: i64 = {
        let count_sql = format!("SELECT COUNT(*) FROM system_logs {}", where_clause);
        if idx > 1 {
            // Build separate params for count (exclude limit/offset)
            let mut count_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            if let Some(t) = query.get("log_type").or_else(|| query.get("type")) {
                count_params.push(Box::new(t.clone()));
            }
            if let Some(l) = query.get("level") {
                count_params.push(Box::new(l.clone()));
            }
            db.query_row(&count_sql, rusqlite::params_from_iter(count_params.iter()), |row| row.get(0)).unwrap_or(0)
        } else {
            db.query_row(&count_sql, [], |row| row.get(0)).unwrap_or(0)
        }
    };

    HttpResponse::Ok().json(AdminResponse {
        success: true,
        data: Some(serde_json::json!(logs)),
        error: None,
        total: Some(total),
    })
}

#[get("/admin/logs/config")]
pub async fn get_log_config(
    state: web::Data<AppState>,
) -> HttpResponse {
    let db = state.db.get().unwrap();

    let log_level = get_config_value(&db, "log_level").unwrap_or_else(|| "info".to_string());
    let log_retention_days = get_config_value(&db, "log_retention_days").unwrap_or_else(|| "7".to_string());

    HttpResponse::Ok().json(AdminResponse {
        success: true,
        data: Some(serde_json::json!({
            "log_level": log_level,
            "log_retention_days": log_retention_days,
        })),
        error: None,
        total: None,
    })
}

#[post("/admin/logs/config")]
pub async fn save_log_config(
    state: web::Data<AppState>,
    body: web::Json<LogConfigRequest>,
) -> HttpResponse {
    let db = state.db.get().unwrap();

    set_config_value(&db, "log_level", &body.log_level);
    set_config_value(&db, "log_retention_days", &body.log_retention_days);

    log_to_db(&db, "system", "info",
        "管理员更新了日志配置",
        Some(&serde_json::json!({"log_level": body.log_level, "log_retention_days": body.log_retention_days}).to_string()),
        None, None);

    HttpResponse::Ok().json(AdminResponse {
        success: true,
        data: Some(serde_json::json!({"message": "日志配置已保存"})),
        error: None,
        total: None,
    })
}

#[post("/admin/logs/clean")]
pub async fn clean_logs(
    state: web::Data<AppState>,
) -> HttpResponse {
    let db = state.db.get().unwrap();

    let retention_days = get_config_value(&db, "log_retention_days")
        .unwrap_or_else(|| "7".to_string())
        .parse::<i64>()
        .unwrap_or(7);

    let result = db.execute(
        "DELETE FROM system_logs WHERE created_at < datetime('now', '-' || ?1 || ' days')",
        rusqlite::params![retention_days],
    );

    match result {
        Ok(count) => {
            log_to_db(&db, "system", "info",
                &format!("管理员清理了过期日志，保留 {} 天，删除 {} 条", retention_days, count),
                None, None, None);
            HttpResponse::Ok().json(AdminResponse {
                success: true,
                data: Some(serde_json::json!({
                    "message": format!("已清理 {} 条过期日志", count),
                    "deleted_count": count,
                    "retention_days": retention_days,
                })),
                error: None,
                total: None,
            })
        }
        Err(e) => HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some(format!("清理失败: {}", e)),
            total: None,
        }),
    }
}

// ════════════════════════════════════════════════════════════════
// 定时管理
// ════════════════════════════════════════════════════════════════

#[get("/admin/cron")]
pub async fn get_cron_tasks(
    state: web::Data<AppState>,
) -> HttpResponse {
    let db = state.db.get().unwrap();

    let mut stmt = db.prepare(
        "SELECT c.id, c.user_id, u.mobile, u.nickname, c.cron_expression, c.status, \
         c.last_run_at, c.last_run_status, c.last_run_message, \
         c.total_runs, c.success_runs, c.failed_runs, c.created_at \
         FROM user_cron_tasks c \
         LEFT JOIN users u ON c.user_id = u.id \
         ORDER BY c.created_at DESC"
    ).unwrap();

    let tasks: Vec<serde_json::Value> = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "user_id": row.get::<_, i64>(1)?,
            "mobile": row.get::<_, Option<String>>(2)?,
            "nickname": row.get::<_, Option<String>>(3)?,
            "cron_expression": row.get::<_, String>(4)?,
            "status": row.get::<_, String>(5)?,
            "last_run_at": row.get::<_, Option<String>>(6)?,
            "last_run_status": row.get::<_, Option<String>>(7)?,
            "last_run_message": row.get::<_, Option<String>>(8)?,
            "total_runs": row.get::<_, i64>(9)?,
            "success_runs": row.get::<_, i64>(10)?,
            "failed_runs": row.get::<_, i64>(11)?,
            "created_at": row.get::<_, String>(12)?,
        }))
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect();

    HttpResponse::Ok().json(AdminResponse {
        success: true,
        data: Some(serde_json::json!(tasks)),
        error: None,
        total: None,
    })
}

#[post("/admin/cron")]
pub async fn create_or_update_cron(
    state: web::Data<AppState>,
    body: web::Json<CronCreateRequest>,
) -> HttpResponse {
    let db = state.db.get().unwrap();

    // 验证用户存在
    let user_exists: bool = db.query_row(
        "SELECT COUNT(*) FROM users WHERE id = ?1",
        rusqlite::params![body.user_id],
        |row| row.get::<_, i64>(0),
    ).unwrap_or(0) > 0;

    if !user_exists {
        return HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some("用户不存在".to_string()),
            total: None,
        });
    }

    // 获取用户 mobile
    let mobile: String = db.query_row(
        "SELECT mobile FROM users WHERE id = ?1",
        rusqlite::params![body.user_id],
        |row| row.get(0),
    ).unwrap_or_default();

    // UPSERT: 如果 user_id 已存在则更新
    let result = db.execute(
        "INSERT INTO user_cron_tasks (user_id, mobile, cron_expression, status) \
         VALUES (?1, ?2, ?3, 'active') \
         ON CONFLICT(user_id) DO UPDATE SET \
         cron_expression = excluded.cron_expression, \
         status = 'active'",
        rusqlite::params![body.user_id, mobile, body.cron_expression],
    );

    match result {
        Ok(_) => {
            log_to_db(&db, "cron", "info",
                &format!("管理员设置了用户 #{} 的定时任务: {}", body.user_id, body.cron_expression),
                None, None, None);
            HttpResponse::Ok().json(AdminResponse {
                success: true,
                data: Some(serde_json::json!({"message": "定时任务已保存"})),
                error: None,
                total: None,
            })
        }
        Err(e) => HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some(format!("保存失败: {}", e)),
            total: None,
        }),
    }
}

#[post("/admin/cron/toggle")]
pub async fn toggle_cron(
    state: web::Data<AppState>,
    body: web::Json<CronToggleRequest>,
) -> HttpResponse {
    let db = state.db.get().unwrap();

    // 获取当前状态
    let current: String = db.query_row(
        "SELECT status FROM user_cron_tasks WHERE id = ?1",
        rusqlite::params![body.id],
        |row| row.get(0),
    ).unwrap_or_default();

    let new_status = if current == "active" { "disabled" } else { "active" };

    let result = db.execute(
        "UPDATE user_cron_tasks SET status = ?1 WHERE id = ?2",
        rusqlite::params![new_status, body.id],
    );

    match result {
        Ok(rows) => {
            if rows > 0 {
                log_to_db(&db, "cron", "info",
                    &format!("管理员{}了定时任务 #{}", if new_status == "active" { "启用" } else { "禁用" }, body.id),
                    None, None, None);
                HttpResponse::Ok().json(AdminResponse {
                    success: true,
                    data: Some(serde_json::json!({
                        "message": if new_status == "active" { "已启用" } else { "已禁用" },
                        "new_status": new_status,
                    })),
                    error: None,
                    total: None,
                })
            } else {
                HttpResponse::Ok().json(AdminResponse {
                    success: false,
                    data: None,
                    error: Some("定时任务不存在".to_string()),
                    total: None,
                })
            }
        }
        Err(e) => HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some(format!("操作失败: {}", e)),
            total: None,
        }),
    }
}

#[delete("/admin/cron")]
pub async fn delete_cron(
    state: web::Data<AppState>,
    body: web::Json<CronDeleteRequest>,
) -> HttpResponse {
    let db = state.db.get().unwrap();

    let result = db.execute(
        "DELETE FROM user_cron_tasks WHERE id = ?1",
        rusqlite::params![body.id],
    );

    match result {
        Ok(rows) => {
            if rows > 0 {
                log_to_db(&db, "cron", "info",
                    &format!("管理员删除了定时任务 #{}", body.id),
                    None, None, None);
                HttpResponse::Ok().json(AdminResponse {
                    success: true,
                    data: Some(serde_json::json!({"message": "已删除"})),
                    error: None,
                    total: None,
                })
            } else {
                HttpResponse::Ok().json(AdminResponse {
                    success: false,
                    data: None,
                    error: Some("定时任务不存在".to_string()),
                    total: None,
                })
            }
        }
        Err(e) => HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some(format!("删除失败: {}", e)),
            total: None,
        }),
    }
}

// ════════════════════════════════════════════════════════════════
// 系统管理
// ════════════════════════════════════════════════════════════════

#[get("/admin/system")]
pub async fn get_system_config(
    state: web::Data<AppState>,
) -> HttpResponse {
    let db = state.db.get().unwrap();

    let guest_register_enabled: i32 = get_config_value(&db, "guest_register_enabled")
        .unwrap_or_else(|| "0".to_string())
        .parse()
        .unwrap_or(0);
    let log_level = get_config_value(&db, "log_level").unwrap_or_else(|| "info".to_string());
    let log_retention_days = get_config_value(&db, "log_retention_days").unwrap_or_else(|| "7".to_string());

    HttpResponse::Ok().json(AdminResponse {
        success: true,
        data: Some(serde_json::json!({
            "guest_register_enabled": guest_register_enabled,
            "log_level": log_level,
            "log_retention_days": log_retention_days,
        })),
        error: None,
        total: None,
    })
}

#[post("/admin/system/password")]
pub async fn change_admin_password(
    state: web::Data<AppState>,
    body: web::Json<ChangePasswordRequest>,
) -> HttpResponse {
    let db = state.db.get().unwrap();

    // 获取当前管理员密码 (默认 admin)
    let stored_password: String = db.query_row(
        "SELECT password FROM admins WHERE id = 1",
        [],
        |row| row.get(0),
    ).unwrap_or_default();

    if !verify(&body.old_password, &stored_password).unwrap_or(false) {
        return HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some("原密码错误".to_string()),
            total: None,
        });
    }

    let hashed = hash(&body.new_password, DEFAULT_COST).unwrap();
    let result = db.execute(
        "UPDATE admins SET password = ?1 WHERE id = 1",
        rusqlite::params![hashed],
    );

    match result {
        Ok(_) => {
            log_to_db(&db, "auth", "warn",
                "管理员修改了密码",
                None, None, None);
            HttpResponse::Ok().json(AdminResponse {
                success: true,
                data: Some(serde_json::json!({"message": "密码已修改"})),
                error: None,
                total: None,
            })
        }
        Err(e) => HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some(format!("修改失败: {}", e)),
            total: None,
        }),
    }
}

#[post("/admin/system/username")]
pub async fn update_admin_username(
    state: web::Data<AppState>,
    body: web::Json<ChangeUsernameRequest>,
) -> HttpResponse {
    let db = state.db.get().unwrap();

    // 检查新用户名是否已存在
    let exists: bool = db.query_row(
        "SELECT COUNT(*) FROM admins WHERE username = ?1",
        rusqlite::params![body.new_username],
        |row| row.get::<_, i64>(0),
    ).unwrap_or(0) > 0;

    if exists {
        return HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some("用户名已存在".to_string()),
            total: None,
        });
    }

    let result = db.execute(
        "UPDATE admins SET username = ?1 WHERE id = 1",
        rusqlite::params![body.new_username],
    );

    match result {
        Ok(_) => {
            log_to_db(&db, "auth", "warn",
                "管理员修改了用户名",
                None, None, None);
            HttpResponse::Ok().json(AdminResponse {
                success: true,
                data: Some(serde_json::json!({"message": "用户名已修改"})),
                error: None,
                total: None,
            })
        }
        Err(e) => HttpResponse::Ok().json(AdminResponse {
            success: false,
            data: None,
            error: Some(format!("修改失败: {}", e)),
            total: None,
        }),
    }
}

#[post("/admin/system/guest-register")]
pub async fn toggle_guest_register(
    state: web::Data<AppState>,
    body: web::Json<GuestRegisterRequest>,
) -> HttpResponse {
    let db = state.db.get().unwrap();

    set_config_value(&db, "guest_register_enabled", &body.enabled);

    let enabled_num: i32 = body.enabled.parse().unwrap_or(0);

    log_to_db(&db, "system", "info",
        &format!("管理员{}了游客注册", if enabled_num == 1 { "启用" } else { "禁用" }),
        None, None, None);

    HttpResponse::Ok().json(AdminResponse {
        success: true,
        data: Some(serde_json::json!({
            "message": format!("游客注册已{}", if enabled_num == 1 { "启用" } else { "禁用" }),
            "guest_register_enabled": enabled_num,
        })),
        error: None,
        total: None,
    })
}

// ════════════════════════════════════════════════════════════════
// 工具函数
// ════════════════════════════════════════════════════════════════

/// 获取 system_config 中的配置值
pub fn get_config_value(db: &rusqlite::Connection, key: &str) -> Option<String> {
    db.query_row(
        "SELECT value FROM system_config WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get(0),
    ).ok()
}

/// 设置 system_config 中的配置值
fn set_config_value(db: &rusqlite::Connection, key: &str, value: &str) {
    let _ = db.execute(
        "INSERT INTO system_config (key, value, updated_at) VALUES (?1, ?2, datetime('now')) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        rusqlite::params![key, value],
    );
}

/// 写日志工具函数
pub fn log_to_db(
    db: &rusqlite::Connection,
    log_type: &str,
    log_level: &str,
    message: &str,
    context: Option<&str>,
    user_id: Option<i64>,
    ip: Option<&str>,
) {
    let _ = db.execute(
        "INSERT INTO system_logs (log_type, log_level, message, context, user_id, ip_address) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![log_type, log_level, message, context, user_id, ip],
    );
}
