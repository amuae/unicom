use actix_web::{get, web, HttpRequest, HttpResponse};
use serde::Serialize;
use chrono::{Datelike, Local};

use crate::AppState;
use crate::models::user::{User, USER_COLUMNS};
use crate::services::query::query_user_flow;
use crate::services::flow_analysis;
use crate::services::notify;

#[derive(Serialize)]
pub struct QueryResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// 从 DB 按 token 查询活跃用户
pub fn query_user_by_token(db: &rusqlite::Connection, token: &str) -> Result<User, rusqlite::Error> {
    db.query_row(
        &format!("SELECT {} FROM users WHERE token = ?1 AND status = 'active'", USER_COLUMNS),
        [token],
        |row| User::from_row(row),
    )
}

/// 从请求中提取用户 token（优先从 Authorization header，回退到 URL path）
pub fn extract_user_token(req: &HttpRequest, path_token: Option<&str>) -> Option<String> {
    // 优先从 header 读取
    if let Some(auth) = req.headers().get("Authorization").and_then(|v| v.to_str().ok()) {
        if auth.starts_with("Bearer ") {
            return Some(auth[7..].to_string());
        }
    }
    // 回退到 URL path token
    path_token.map(|s| s.to_string())
}

/// 执行查询并分析流量数据（核心逻辑，供多个 handler 复用）
pub async fn perform_query_analysis(
    state: &web::Data<AppState>,
    token: &str,
) -> Result<serde_json::Value, String> {
    // ── Phase 1: 获取用户 ──
    let user = {
        let db = state.db.get().map_err(|e| format!("数据库连接失败: {}", e))?;
        query_user_by_token(&db, token)
            .map_err(|_| "用户不存在或已被禁用".to_string())?
    };

    // ── Phase 2: 执行原始联通 API 查询 ──
    let result = query_user_flow(state, &user).await
        .map_err(|e| e.to_string())?;

    // ── Phase 3: 更新 cookie ──
    {
        let db = state.db.get().map_err(|e| format!("数据库连接失败: {}", e))?;
        if result.need_update_cookie {
            db.execute(
                "UPDATE users SET cookie = ?1, cookie_created_at = ?2, \
                 last_query_at = datetime('now', 'localtime') WHERE id = ?3",
                rusqlite::params![result.cookie, result.cookie_created_at, user.id],
            ).map_err(|e| e.to_string())?;
        } else {
            db.execute(
                "UPDATE users SET last_query_at = datetime('now', 'localtime') WHERE id = ?1",
                [user.id],
            ).map_err(|e| e.to_string())?;
        }
    }

    // ── Phase 4: 分析流量数据（9 桶） ──
    let flow_json = serde_json::to_value(&result.flow_data)
        .map_err(|e| format!("序列化流量数据失败: {}", e))?;
    let mut analysis = flow_analysis::analyze_flow(&flow_json, &user.mobile);

    // ── Phase 5: 计算差值 ──
    let last_time_str = user.last_query_time
        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default();
    // notify_diff: 与上次通知时的数据对比
    let notify_diff = flow_analysis::calculate_diff(
        &analysis,
        &user.last_query_data,
        &last_time_str,
    );
    // daily_diff: 与今日首次查询的数据对比
    let daily_diff = flow_analysis::calculate_diff(
        &analysis,
        &user.today_query_data,
        &last_time_str,
    );
    analysis.diff = notify_diff.clone();

    // ── Phase 6: 设置时间间隔 ──
    flow_analysis::set_time_interval(&mut analysis, &last_time_str);

    // ── Phase 7: 设置余额信息 ──
    if let Some(balance_data) = &result.balance_data {
        analysis.balance = Some(flow_analysis::BalanceInfo {
            balance: balance_data.balance.clone(),
            real_fee: balance_data.real_fee.clone(),
            can_use_fee: balance_data.can_use_fee.clone(),
        });
    }

    // ── Phase 8: 检查通知阈值 ──
    let threshold = user.notify_threshold as f64;
    let should_notify = if threshold > 0.0 {
        analysis.diff.get("所有通用")
            .map(|d| d.uused >= threshold)
            .unwrap_or(false)
    } else {
        false
    };

    // 发送通知（在 DB 锁外执行，避免持锁进行网络 IO）
    if should_notify {
        if let Err(e) = notify::send_flow_notify(
            &user,
            &analysis,
            &notify_diff,
            &daily_diff,
            &analysis.time_interval,
        ).await {
            tracing::warn!("发送通知失败: {}", e);
        }
    }

    // ── Phase 9: 保存分析结果到 DB ──
    let analysis_json = serde_json::to_string(&analysis).unwrap_or_default();
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    {
        let db = state.db.get().map_err(|e| format!("数据库连接失败: {}", e))?;

        // 检查是否跨月
        let is_cross_month = if let Some(last_time) = user.last_query_time {
            let now = Local::now();
            let current_month = now.year() * 12 + now.month() as i32;
            let previous_month = last_time.year() * 12 + last_time.month() as i32;
            current_month > previous_month
        } else {
            false
        };

        // 是否需要更新 last_query_data（基线数据）
        let update_last = user.last_query_data.is_empty() || is_cross_month || should_notify;

        if update_last {
            db.execute(
                "UPDATE users SET today_query_data = ?1, last_query_data = ?1, \
                 last_query_time = ?2 WHERE id = ?3",
                rusqlite::params![analysis_json, now_str, user.id],
            ).map_err(|e| e.to_string())?;
        } else {
            db.execute(
                "UPDATE users SET today_query_data = ?1 WHERE id = ?2",
                rusqlite::params![analysis_json, user.id],
            ).map_err(|e| e.to_string())?;
        }
    }

    // ── Phase 10: 返回完整分析结果 ──
    serde_json::to_value(&analysis)
        .map_err(|e| format!("序列化分析结果失败: {}", e))
}

// ─────────────────────────────────────────────
// API 端点（旧版 URL token，保留兼容）
// ─────────────────────────────────────────────

#[get("/query/flow/{token}")]
pub async fn query_flow(
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

#[get("/query/balance/{token}")]
pub async fn query_balance(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let token = path.into_inner();
    let user = {
        let db = match state.db.get() {
            Ok(db) => db,
            Err(e) => return HttpResponse::InternalServerError().json(QueryResponse {
                success: false,
                data: None,
                error: Some(format!("数据库连接失败: {}", e)),
            }),
        };
        query_user_by_token(&db, &token)
    };

    let user = match user {
        Ok(user) => user,
        Err(_) => {
            return HttpResponse::NotFound().json(QueryResponse {
                success: false,
                data: None,
                error: Some("用户不存在或已被禁用".to_string()),
            });
        }
    };

    let unicom_service = match crate::services::unicom::UnicomService::new(state.config.unicom.clone()) {
        Ok(s) => s,
        Err(e) => return HttpResponse::InternalServerError().json(QueryResponse {
            success: false,
            data: None,
            error: Some(format!("创建 HTTP 客户端失败: {}", e)),
        }),
    };

    match unicom_service.query_balance(&user.cookie).await {
        Ok(balance) => HttpResponse::Ok().json(QueryResponse {
            success: true,
            data: Some(serde_json::json!(balance)),
            error: None,
        }),
        Err(e) => HttpResponse::InternalServerError().json(QueryResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        }),
    }
}
