use actix_web::{post, web, HttpRequest, HttpResponse};
use serde::Deserialize;
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;
use std::collections::HashMap;
use std::time::Instant;

use crate::AppState;
use crate::handlers::admin::log_to_db;
use crate::models::user::{User, USER_COLUMNS};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Admin session 缓存：token → (admin_id, 缓存时间)
pub struct SessionCache {
    entries: HashMap<String, (i64, Instant)>,
    ttl_secs: u64,
}

impl SessionCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            entries: HashMap::new(),
            ttl_secs,
        }
    }

    /// 查询缓存，返回 Some(admin_id) 表示命中
    pub fn get(&self, token: &str) -> Option<i64> {
        self.entries.get(token).and_then(|(id, cached_at)| {
            if cached_at.elapsed().as_secs() < self.ttl_secs {
                Some(*id)
            } else {
                None
            }
        })
    }

    /// 写入缓存
    pub fn insert(&mut self, token: String, admin_id: i64) {
        self.entries.insert(token, (admin_id, Instant::now()));
    }

    /// 移除缓存
    pub fn remove(&mut self, token: &str) {
        self.entries.remove(token);
    }

    /// 清理过期条目
    pub fn cleanup(&mut self) {
        let ttl = self.ttl_secs;
        self.entries.retain(|_, (_, cached_at)| {
            cached_at.elapsed().as_secs() < ttl
        });
    }

    /// 清理指定 admin 的所有缓存（登录时调用）
    pub fn remove_by_admin(&mut self, admin_id: i64) {
        self.entries.retain(|_, (id, _)| *id != admin_id);
    }
}

#[post("/auth/login")]
pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let db = match state.db.get() {
        Ok(db) => db,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("数据库连接失败: {}", e)
        })),
    };
    let ip = req.peer_addr().map(|a| a.to_string()).unwrap_or_default();

    // 速率限制
    {
        let mut limiter = state.rate_limiter.lock().unwrap();
        if !limiter.check(&ip) {
            return HttpResponse::TooManyRequests().json(serde_json::json!({
                "error": "登录尝试过于频繁，请稍后再试"
            }));
        }
    }

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
        Ok((admin_id, username, stored_password)) => {
            if verify(&body.password, &stored_password).unwrap_or(false) {
                // 清理该管理员的旧 session
                let _ = db.execute(
                    "DELETE FROM admin_sessions WHERE admin_id = ?1",
                    rusqlite::params![admin_id],
                );
                {
                    let mut cache = state.session_cache.lock().unwrap();
                    cache.remove_by_admin(admin_id);
                }

                // 生成新 session token
                let session_token = Uuid::new_v4().to_string();
                let _ = db.execute(
                    "INSERT INTO admin_sessions (token, admin_id, expires_at) \
                     VALUES (?1, ?2, datetime('now', '+24 hours'))",
                    rusqlite::params![session_token, admin_id],
                );
                let _ = db.execute(
                    "UPDATE admins SET last_login_at = datetime('now') WHERE id = ?1",
                    rusqlite::params![admin_id],
                );

                // 写入缓存
                {
                    let mut cache = state.session_cache.lock().unwrap();
                    cache.insert(session_token.clone(), admin_id);
                }

                log_to_db(&db, "auth", "info",
                    &format!("管理员 {} 登录成功", username),
                    None, Some(admin_id), Some(&ip));
                HttpResponse::Ok().json(serde_json::json!({
                    "token": session_token,
                    "user_id": admin_id,
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

/// 从请求头验证 admin session token，返回 admin_id
/// 优先查内存缓存，缓存未命中再查 DB
pub fn verify_admin_token(state: &AppState, req: &HttpRequest) -> Result<i64, HttpResponse> {
    let auth_header = req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let token = if auth_header.starts_with("Bearer ") {
        &auth_header[7..]
    } else {
        return Err(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "缺少认证信息"
        })));
    };

    // 先查内存缓存
    {
        let cache = state.session_cache.lock().unwrap();
        if let Some(admin_id) = cache.get(token) {
            return Ok(admin_id);
        }
    }

    // 缓存未命中，查 DB
    let db = match state.db.get() {
        Ok(db) => db,
        Err(_) => return Err(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "数据库连接失败"
        }))),
    };

    let result = db.query_row(
        "SELECT admin_id FROM admin_sessions \
         WHERE token = ?1 AND expires_at > datetime('now')",
        [token],
        |row| row.get::<_, i64>(0),
    );

    match result {
        Ok(admin_id) => {
            // 写入缓存
            let mut cache = state.session_cache.lock().unwrap();
            cache.insert(token.to_string(), admin_id);
            Ok(admin_id)
        }
        Err(_) => Err(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "认证已过期或无效，请重新登录"
        }))),
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
    let db = match state.db.get() {
        Ok(db) => db,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("数据库连接失败: {}", e)
        })),
    };

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

    let hashed_password = hash(&body.password, DEFAULT_COST).unwrap();

    match db.execute(
        "INSERT INTO admins (username, password, real_name) VALUES (?1, ?2, ?3)",
        rusqlite::params![body.username, hashed_password, body.real_name.as_deref().unwrap_or("")],
    ) {
        Ok(_) => {
            log_to_db(&db, "auth", "info",
                &format!("新管理员注册: {}", body.username),
                None, None, None);
            HttpResponse::Ok().json(serde_json::json!({
                "message": "注册成功"
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("注册失败: {}", e)
        })),
    }
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
    let db = match state.db.get() {
        Ok(db) => db,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("数据库连接失败: {}", e)
        })),
    };
    let ip = req.peer_addr().map(|a| a.to_string()).unwrap_or_default();

    // 速率限制
    {
        let mut limiter = state.rate_limiter.lock().unwrap();
        if !limiter.check(&format!("user_{}", ip)) {
            return HttpResponse::TooManyRequests().json(serde_json::json!({
                "success": false,
                "error": "登录尝试过于频繁，请稍后再试"
            }));
        }
    }

    let user_result = db.query_row(
        &format!("SELECT {} FROM users WHERE mobile = ?1", USER_COLUMNS),
        [&body.mobile],
        |row| User::from_row(row),
    );

    match user_result {
        Ok(user) => {
            // 优先用 bcrypt 验证 login_password_hash
            if !user.login_password_hash.is_empty() {
                if verify(&body.query_password, &user.login_password_hash).unwrap_or(false) {
                    log_to_db(&db, "auth", "info",
                        &format!("用户 {} ({}) 登录成功", user.nickname, user.mobile),
                        None, Some(user.id), Some(&ip));
                    return HttpResponse::Ok().json(serde_json::json!({
                        "success": true,
                        "data": {
                            "token": user.token,
                            "mobile": user.mobile,
                            "nickname": user.nickname,
                        }
                    }));
                }
            }

            // 兼容旧用户：回退到明文 query_password 比较
            if user.query_password == body.query_password {
                log_to_db(&db, "auth", "info",
                    &format!("用户 {} ({}) 登录成功 (明文兼容)", user.nickname, user.mobile),
                    None, Some(user.id), Some(&ip));
                HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "data": {
                        "token": user.token,
                        "mobile": user.mobile,
                        "nickname": user.nickname,
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
    let db = match state.db.get() {
        Ok(db) => db,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("数据库连接失败: {}", e)
        })),
    };
    let ip = req.peer_addr().map(|a| a.to_string()).unwrap_or_default();

    let guest_enabled = crate::handlers::admin::get_config_value(&db, "guest_register_enabled")
        .unwrap_or_else(|| "0".to_string());
    if guest_enabled != "1" {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "游客注册未开放"
        }));
    }

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

    let token = Uuid::new_v4().to_string();
    let nickname = body.nickname.as_deref().unwrap_or("");
    let auth_type = body.auth_type.as_deref().unwrap_or("cookie");
    let cookie = body.cookie.as_deref().unwrap_or("");
    let appid = body.appid.as_deref().unwrap_or("");
    let token_online = body.token_online.as_deref().unwrap_or("");

    // bcrypt 哈希用于登录验证
    let password_hash = hash(&body.query_password, DEFAULT_COST).unwrap();

    let result = db.execute(
        "INSERT INTO users (mobile, nickname, query_password, login_password_hash, auth_type, cookie, appid, token_online, token, status, \
         today_query_data, last_query_data) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'active', '', '')",
        rusqlite::params![body.mobile, nickname, body.query_password, password_hash, auth_type, cookie, appid, token_online, token],
    );

    match result {
        Ok(_) => {
            let user_id: i64 = db.query_row(
                "SELECT id FROM users WHERE mobile = ?1",
                [&body.mobile],
                |row| row.get(0),
            ).unwrap_or(0);

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
