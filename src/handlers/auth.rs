use actix_web::{post, web, HttpRequest, HttpResponse};
use serde::Deserialize;
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;

use crate::AppState;
use crate::handlers::admin::log_to_db;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[post("/auth/login")]
pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let db = state.db.get().unwrap();
    let ip = req.peer_addr().map(|a| a.to_string()).unwrap_or_default();
    
    // 查询管理员
    let result = db.query_row(
        "SELECT id, username, password FROM admins WHERE username = ?1",
        [&body.username],
        |row| {
            let id: i64 = row.get(0)?;
            let username: String = row.get(1)?;
            let password: String = row.get(2)?;
            Ok((id, username, password))
        },
    );

    match result {
        Ok((user_id, username, stored_password)) => {
            if verify(&body.password, &stored_password).unwrap_or(false) {
                let token = Uuid::new_v4().to_string();
                // 更新最后登录时间
                let _ = db.execute(
                    "UPDATE admins SET last_login_at = datetime('now') WHERE id = ?1",
                    rusqlite::params![user_id],
                );
                log_to_db(&db, "auth", "info",
                    &format!("管理员 {} 登录成功", username),
                    None, Some(user_id), Some(&ip));
                HttpResponse::Ok().json(serde_json::json!({
                    "token": token,
                    "user_id": user_id,
                    "username": username
                }))
            } else {
                log_to_db(&db, "auth", "warn",
                    &format!("管理员 {} 登录失败: 密码错误", body.username),
                    None, None, Some(&ip));
                HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "用户名或密码错误"
                }))
            }
        }
        Err(_) => {
            log_to_db(&db, "auth", "warn",
                &format!("管理员登录失败: 用户名 {} 不存在", body.username),
                None, None, Some(&ip));
            HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "用户名或密码错误"
            }))
        }
    }
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub real_name: Option<String>,
}

#[post("/auth/register")]
pub async fn register(
    state: web::Data<AppState>,
    body: web::Json<RegisterRequest>,
) -> HttpResponse {
    let db = state.db.get().unwrap();
    
    // 检查用户名是否已存在
    let exists: bool = db
        .query_row(
            "SELECT COUNT(*) FROM admins WHERE username = ?1",
            [&body.username],
            |row| row.get(0),
        )
        .unwrap_or(0) > 0;

    if exists {
        return HttpResponse::Conflict().json(serde_json::json!({
            "error": "用户名已存在"
        }));
    }

    // 哈希密码
    let hashed_password = hash(&body.password, DEFAULT_COST).unwrap();

    // 插入管理员
    db.execute(
        "INSERT INTO admins (username, password, real_name) VALUES (?1, ?2, ?3)",
        rusqlite::params![body.username, hashed_password, body.real_name.as_deref().unwrap_or("")],
    ).unwrap();

    log_to_db(&db, "auth", "info",
        &format!("新管理员注册: {}", body.username),
        None, None, None);

    HttpResponse::Ok().json(serde_json::json!({
        "message": "注册成功"
    }))
}

// ════════════════════════════════════════════════════════════════
// 用户登录 / 注册
// ════════════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct UserLoginRequest {
    pub mobile: String,
    pub query_password: String,
}

#[post("/auth/user-login")]
pub async fn user_login(
    state: web::Data<AppState>,
    body: web::Json<UserLoginRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let db = state.db.get().unwrap();
    let ip = req.peer_addr().map(|a| a.to_string()).unwrap_or_default();

    let result = db.query_row(
        "SELECT id, mobile, nickname, token, query_password FROM users WHERE mobile = ?1",
        [&body.mobile],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        },
    );

    match result {
        Ok((user_id, mobile, nickname, token, stored_password)) => {
            if stored_password == body.query_password {
                log_to_db(&db, "auth", "info",
                    &format!("用户 {} ({}) 登录成功", nickname, mobile),
                    None, Some(user_id), Some(&ip));
                HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "data": {
                        "token": token,
                        "mobile": mobile,
                        "nickname": nickname,
                    }
                }))
            } else {
                log_to_db(&db, "auth", "warn",
                    &format!("用户 {} 登录失败: 密码错误", body.mobile),
                    None, None, Some(&ip));
                HttpResponse::Unauthorized().json(serde_json::json!({
                    "success": false,
                    "error": "手机号或密码错误"
                }))
            }
        }
        Err(_) => {
            log_to_db(&db, "auth", "warn",
                &format!("用户登录失败: 手机号 {} 不存在", body.mobile),
                None, None, Some(&ip));
            HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "手机号或密码错误"
            }))
        }
    }
}

#[derive(Deserialize)]
pub struct UserRegisterRequest {
    pub mobile: String,
    pub query_password: String,
    pub nickname: Option<String>,
    pub auth_type: Option<String>,
    pub cookie: Option<String>,
    pub appid: Option<String>,
    pub token_online: Option<String>,
}

#[post("/auth/user-register")]
pub async fn user_register(
    state: web::Data<AppState>,
    body: web::Json<UserRegisterRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let db = state.db.get().unwrap();
    let ip = req.peer_addr().map(|a| a.to_string()).unwrap_or_default();

    // 检查游客注册是否开启
    let guest_enabled = crate::handlers::admin::get_config_value(&db, "guest_register_enabled")
        .unwrap_or_else(|| "0".to_string());
    if guest_enabled != "1" {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "游客注册未开放"
        }));
    }

    // 检查手机号是否已存在
    let exists: bool = db.query_row(
        "SELECT COUNT(*) FROM users WHERE mobile = ?1",
        [&body.mobile],
        |row| row.get::<_, i64>(0),
    ).unwrap_or(0) > 0;

    if exists {
        return HttpResponse::Conflict().json(serde_json::json!({
            "success": false,
            "error": "手机号已注册"
        }));
    }

    // 生成 token
    let token = Uuid::new_v4().to_string();
    let nickname = body.nickname.as_deref().unwrap_or("");
    let auth_type = body.auth_type.as_deref().unwrap_or("cookie");
    let cookie = body.cookie.as_deref().unwrap_or("");
    let appid = body.appid.as_deref().unwrap_or("");
    let token_online = body.token_online.as_deref().unwrap_or("");

    let result = db.execute(
        "INSERT INTO users (mobile, nickname, query_password, auth_type, cookie, appid, token_online, token, status, \
         today_query_data, last_query_data) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active', '', '')",
        rusqlite::params![body.mobile, nickname, body.query_password, auth_type, cookie, appid, token_online, token],
    );

    match result {
        Ok(_) => {
            // 获取新用户 ID
            let user_id: i64 = db.query_row(
                "SELECT id FROM users WHERE mobile = ?1",
                [&body.mobile],
                |row| row.get(0),
            ).unwrap_or(0);

            // 自动创建默认定时任务（每5分钟）
            let _ = db.execute(
                "INSERT INTO user_cron_tasks (user_id, mobile, cron_expression, status) \
                 VALUES (?1, ?2, '0 */5 * * * *', 'active')",
                rusqlite::params![user_id, body.mobile],
            );

            log_to_db(&db, "auth", "info",
                &format!("新用户注册: {} ({})", nickname, body.mobile),
                None, Some(user_id), Some(&ip));

            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "data": {
                    "message": "注册成功",
                    "token": token,
                    "mobile": body.mobile,
                    "nickname": nickname,
                }
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("注册失败: {}", e)
        })),
    }
}