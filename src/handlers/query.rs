use actix_web::{get, web, HttpResponse};
use serde::Serialize;
use chrono::{Datelike, Local};

use crate::AppState;
use crate::models::user::User;
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
        "SELECT id, mobile, nickname, query_password, auth_type, appid, token_online, cookie, cookie_created_at, \
         status, last_query_at, created_at, updated_at, \
         COALESCE(today_query_data, ''), COALESCE(last_query_data, ''), last_query_time, \
         token, notify_enabled, notify_type, notify_params, notify_title, notify_subtitle, notify_content, \
         notify_threshold, query_interval, last_notify_time \
         FROM users WHERE token = ?1 AND status = 'active'",
        [token],
        |row| {
            Ok(User {
                id: row.get(0)?,
                mobile: row.get(1)?,
                nickname: row.get(2)?,
                query_password: row.get(3)?,
                auth_type: row.get(4)?,
                appid: row.get(5)?,
                token_online: row.get(6)?,
                cookie: row.get(7)?,
                cookie_created_at: row.get(8)?,
                status: row.get(9)?,
                last_query_at: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
                today_query_data: row.get(13)?,
                last_query_data: row.get(14)?,
                last_query_time: row.get(15)?,
                token: row.get(16)?,
                notify_enabled: row.get(17)?,
                notify_type: row.get(18)?,
                notify_params: row.get(19)?,
                notify_title: row.get(20)?,
                notify_subtitle: row.get(21)?,
                notify_content: row.get(22)?,
                notify_threshold: row.get(23)?,
                query_interval: row.get(24)?,
                last_notify_time: row.get(25)?,
            })
        },
    )
}

/// 执行查询并分析流量数据（核心逻辑，供多个 handler 复用）
pub async fn perform_query_analysis(
    state: &web::Data<AppState>,
    token: &str,
) -> Result<serde_json::Value, String> {
    // ── Phase 1: 获取用户 ──
    let user = {
        let db = state.db.get().unwrap();
        query_user_by_token(&db, token)
            .map_err(|_| "用户不存在或已被禁用".to_string())?
    };

    // ── Phase 2: 执行原始联通 API 查询 ──
    let result = query_user_flow(state, &user).await
        .map_err(|e| e.to_string())?;

    // ── Phase 3: 更新 cookie ──
    {
        let db = state.db.get().unwrap();
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
        let db = state.db.get().unwrap();

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
        // 条件：首次 / 跨月 / 达到阈值
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
// API 端点
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
        Err(e) => HttpResponse::InternalServerError().json(QueryResponse {
            success: false,
            data: None,
            error: Some(e),
        }),
    }
}

#[get("/query/balance/{token}")]
pub async fn query_balance(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let token = path.into_inner();
    let user = {
        let db = state.db.get().unwrap();
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

    let unicom_service = crate::services::unicom::UnicomService::new(state.config.unicom.clone()).unwrap();

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