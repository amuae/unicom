use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, error, warn, debug};
use chrono::{Datelike, Local, Timelike};

use crate::AppState;
use crate::models::user::User;
use crate::services::query::query_user_flow;
use crate::services::flow_analysis::{self, FlowDiff};
use crate::services::notify;
use crate::handlers::admin::log_to_db;

pub async fn start_cron_jobs(state: AppState) {
    let sched = JobScheduler::new().await.unwrap();

    // 每分钟检查一次 user_cron_tasks 中 status=active 的任务
    let state_clone = state.clone();
    let job = Job::new("0 * * * * *", move |_, _| {
        let state = state_clone.clone();
        tokio::spawn(async move {
            if let Err(e) = check_and_run_cron_tasks(state).await {
                error!("定时任务检查失败: {}", e);
            }
        });
    })
    .unwrap();

    sched.add(job).await.unwrap();

    // 启动调度器
    sched.start().await.unwrap();
    info!("定时任务调度器已启动（基于 user_cron_tasks 表）");
}

/// 从 user_cron_tasks 表读取 active 任务，检查是否到了执行时间
async fn check_and_run_cron_tasks(state: AppState) -> anyhow::Result<()> {
    // 查询所有 active 的定时任务
    let tasks: Vec<(i64, i64, String, String)> = {
        let db = state.db.get().unwrap();
        let mut stmt = db.prepare(
            "SELECT c.id, c.user_id, c.mobile, c.cron_expression \
             FROM user_cron_tasks c \
             INNER JOIN users u ON c.user_id = u.id \
             WHERE c.status = 'active' AND u.status = 'active'"
        )?;

        let tasks = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();
        
        tasks
    };

    debug!("定时任务检查: 找到 {} 个活跃任务，开始并发执行", tasks.len());

    let now = Local::now();
    let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

    // 并发执行所有到期任务
    let mut handles = Vec::new();
    for (task_id, user_id, mobile, cron_expr) in tasks {
        if !should_run_now(&cron_expr, &now) {
            continue;
        }

        info!("定时任务 #{}: 用户 {} ({}) 到达执行时间", task_id, mobile, cron_expr);

        let state_clone = state.clone();
        let now_str_clone = now_str.clone();
        handles.push(tokio::spawn(async move {
            // 获取完整用户信息
            let user = {
                let db = state_clone.db.get().unwrap();
                get_user_by_id(&db, user_id)
            };

            let user = match user {
                Some(u) => u,
                None => {
                    warn!("定时任务 #{}: 用户 #{} 不存在，跳过", task_id, user_id);
                    return;
                }
            };

            // 执行任务
            let result = process_single_user(&state_clone, &user).await;

            // 更新定时任务执行记录
            let (status, message) = match &result {
                Ok(_) => ("success".to_string(), "查询成功".to_string()),
                Err(e) => ("failed".to_string(), e.to_string()),
            };

            {
                let db = state_clone.db.get().unwrap();
                let _ = db.execute(
                    "UPDATE user_cron_tasks SET \
                     last_run_at = ?1, last_run_status = ?2, last_run_message = ?3, \
                     total_runs = total_runs + 1, \
                     success_runs = CASE WHEN ?2 = 'success' THEN success_runs + 1 ELSE success_runs END, \
                     failed_runs = CASE WHEN ?2 = 'failed' THEN failed_runs + 1 ELSE failed_runs END \
                     WHERE id = ?4",
                    rusqlite::params![now_str_clone, status, message, task_id],
                );

                // 写入系统日志
                log_to_db(&db, "cron", if status == "success" { "info" } else { "error" },
                    &format!("定时任务 #{}: 用户 {} ({}) - {}", task_id, user.nickname, user.mobile, message),
                    None, Some(user_id), None);
            }
        }));
    }

    // 等待所有任务完成
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}

/// 简单的 cron 表达式匹配（6位：秒 分 时 日 月 周）
fn should_run_now(cron_expr: &str, now: &chrono::DateTime<Local>) -> bool {
    let parts: Vec<&str> = cron_expr.split_whitespace().collect();
    if parts.len() < 5 {
        return false;
    }

    // tokio-cron-scheduler 使用 6 位格式
    // parts[0] = seconds, parts[1] = minutes, parts[2] = hours, ...
    let (sec_idx, min_idx, hour_idx) = if parts.len() >= 6 {
        (0, 1, 2)
    } else {
        // 5位格式：分 时 日 月 周
        (99, 0, 1) // 99 = skip seconds check
    };

    let sec = now.second() as i32;
    let min = now.minute() as i32;
    let hour = now.hour() as i32;

    // 检查分钟
    if !cron_matches(parts[min_idx], min, 0, 59) {
        return false;
    }
    // 检查小时
    if !cron_matches(parts[hour_idx], hour, 0, 23) {
        return false;
    }
    // 如果有秒字段，检查秒
    if sec_idx < parts.len() {
        if !cron_matches(parts[sec_idx], sec, 0, 59) {
            return false;
        }
    }

    true
}

/// 匹配单个 cron 字段
fn cron_matches(field: &str, value: i32, min: i32, max: i32) -> bool {
    if field == "*" {
        return true;
    }

    // 处理 */N 格式（每 N 分钟/秒）
    if field.starts_with("*/") {
        if let Ok(step) = field[2..].parse::<i32>() {
            return step > 0 && (value - min) % step == 0;
        }
        return false;
    }

    // 处理具体数值
    if let Ok(n) = field.parse::<i32>() {
        return n >= min && n <= max && value == n;
    }

    // 处理逗号分隔的多个值
    for part in field.split(',') {
        if let Ok(n) = part.trim().parse::<i32>() {
            if value == n {
                return true;
            }
        }
    }

    false
}

/// 通过 user_id 获取完整的 User 对象
fn get_user_by_id(db: &rusqlite::Connection, user_id: i64) -> Option<User> {
    db.query_row(
        "SELECT id, mobile, nickname, query_password, auth_type, appid, token_online, \
         cookie, cookie_created_at, status, last_query_at, created_at, updated_at, \
         COALESCE(today_query_data, ''), COALESCE(last_query_data, ''), last_query_time, token, \
         notify_enabled, notify_type, notify_params, notify_title, notify_subtitle, \
         notify_content, notify_threshold, query_interval, last_notify_time \
         FROM users WHERE id = ?1",
        rusqlite::params![user_id],
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
    ).ok()
}

/// 处理单个用户的查询+分析+通知流程
async fn process_single_user(state: &AppState, user: &User) -> anyhow::Result<()> {
    let now = Local::now();
    let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let today_str = now.format("%Y-%m-%d").to_string();
    let current_month = now.year() * 12 + now.month() as i32;

    // ── 1. 调用查询服务获取流量数据 ──
    let result = match query_user_flow(state, user).await {
        Ok(r) => r,
        Err(e) => {
            let err_msg = e.to_string();
            // 判断是否为凭证失效（Cookie 失效、登录失败等）
            if is_credential_error(&err_msg) {
                warn!("用户 {} 凭证失效: {}", user.mobile, err_msg);
                // 发送凭证失效通知
                if let Err(ne) = notify::send_credential_expired_notify(user, &err_msg).await {
                    error!("发送凭证失效通知失败: {}", ne);
                }
                // 更新数据库：标记查询失败
                if let Err(ue) = update_user_query_state(state, user, &err_msg).await {
                    error!("更新用户查询状态失败: {}", ue);
                }
            }
            return Err(e);
        }
    };

    // ── 2. 将 FlowData 转为 serde_json::Value 供分析 ──
    let flow_json: serde_json::Value = serde_json::to_value(&result.flow_data)
        .map_err(|e| anyhow::anyhow!("序列化 FlowData 失败: {}", e))?;

    // ── 3. 调用 analyze_flow 进行 9 桶分析 ──
    let mut analysis = flow_analysis::analyze_flow(&flow_json, &user.mobile);

    // 设置时间间隔
    let last_query_time_str = user.last_query_time
        .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default();
    flow_analysis::set_time_interval(&mut analysis, &last_query_time_str);

    // 设置余额信息
    if let Some(ref balance_data) = result.balance_data {
        flow_analysis::set_balance(
            &mut analysis,
            flow_analysis::BalanceInfo {
                balance: balance_data.balance.clone(),
                real_fee: balance_data.real_fee.clone(),
                can_use_fee: balance_data.can_use_fee.clone(),
            },
        );
    }

    // ── 4. 决定使用哪个 previous data 来计算 diff ──
    let is_cross_day = user.last_query_time
        .map(|t| t.format("%Y-%m-%d").to_string() != today_str)
        .unwrap_or(true);

    let is_cross_month = user.last_query_time
        .map(|t| {
            let prev_month = t.year() * 12 + t.month() as i32;
            current_month > prev_month
        })
        .unwrap_or(true);

    let (previous_json, previous_time) = if is_cross_day {
        (user.last_query_data.as_str(), last_query_time_str.as_str())
    } else {
        (user.today_query_data.as_str(), last_query_time_str.as_str())
    };

    // ── 5. 计算 diff（用于展示） ──
    let diff = flow_analysis::calculate_diff(&analysis, previous_json, previous_time);

    // ── 5b. 计算通知用的 diff（始终基于 last_query_data，即上次通知重置后的基准）──
    let notify_diff = if !user.last_query_data.is_empty() && !last_query_time_str.is_empty() {
        flow_analysis::calculate_diff(&analysis, &user.last_query_data, &last_query_time_str)
    } else {
        diff.clone()
    };

    // ── 5c. 计算今日累计 diff（基于 today_query_data）──
    let daily_diff = if !user.today_query_data.is_empty() {
        flow_analysis::calculate_diff(&analysis, &user.today_query_data, &last_query_time_str)
    } else {
        diff.clone()
    };

    // ── 6. 构建要保存的分析数据 JSON ──
    analysis.diff = diff.clone();
    let analysis_json = serde_json::to_string(&analysis)
        .unwrap_or_else(|_| "{}".to_string());

    // ── 7. 判断保存策略 ──
    let new_today_data = analysis_json.clone();
    let new_last_data = if is_cross_month {
        analysis_json.clone()
    } else {
        user.last_query_data.clone()
    };

    // ── 8. 保存到数据库 ──
    {
        let db = state.db.get().unwrap();
        if is_cross_month {
            // 跨月：重置基准，更新 last_query_time
            db.execute(
                "UPDATE users SET \
                    cookie = ?1, \
                    cookie_created_at = ?2, \
                    last_query_at = datetime('now', 'localtime'), \
                    last_query_time = datetime('now', 'localtime'), \
                    today_query_data = ?3, \
                    last_query_data = ?4 \
                 WHERE id = ?5",
                rusqlite::params![
                    result.cookie,
                    result.cookie_created_at,
                    new_today_data,
                    new_last_data,
                    user.id,
                ],
            )?;
        } else {
            // 正常：只更新 today_query_data，不更新 last_query_time
            db.execute(
                "UPDATE users SET \
                    cookie = ?1, \
                    cookie_created_at = ?2, \
                    last_query_at = datetime('now', 'localtime'), \
                    today_query_data = ?3 \
                 WHERE id = ?4",
                rusqlite::params![
                    result.cookie,
                    result.cookie_created_at,
                    new_today_data,
                    user.id,
                ],
            )?;
        }
    }

    info!(
        "用户 {} ({}) 查询完成，数据已保存",
        user.nickname, user.mobile
    );

    // ── 9. 检查是否需要发送通知 ──
    if user.notify_enabled == 1 && !user.notify_type.is_empty() {
        let should_notify = check_notify_threshold(user, &notify_diff);

        if should_notify {
            info!(
                "用户 {} ({}) 达到通知阈值，发送通知",
                user.nickname, user.mobile
            );

            if let Err(e) = notify::send_flow_notify(
                user,
                &analysis,
                &notify_diff,
                &daily_diff,
                &analysis.time_interval,
            )
            .await
            {
                error!("发送流量通知失败: {}", e);
            }

            {
                let db = state.db.get().unwrap();
                db.execute(
                    "UPDATE users SET \
                        last_query_data = ?1, \
                        last_query_time = ?2, \
                        last_notify_time = ?2 \
                     WHERE id = ?3",
                    rusqlite::params![
                        analysis_json,
                        now_str,
                        user.id,
                    ],
                )?;
            }

            info!("用户 {} ({}) 通知已发送，last_query_data 已重置", user.nickname, user.mobile);
        }
    }

    Ok(())
}

/// 判断错误是否为凭证失效
fn is_credential_error(err_msg: &str) -> bool {
    err_msg.contains("Cookie已失效")
        || err_msg.contains("Cookie 为空")
        || err_msg.contains("Cookie为空")
        || err_msg.contains("缺少登录凭证")
        || err_msg.contains("未获取到Cookie")
        || err_msg.contains("登录失败")
        || err_msg.contains("登录请求失败")
}

/// 更新用户查询失败状态
async fn update_user_query_state(state: &AppState, user: &User, _err_msg: &str) -> anyhow::Result<()> {
    let db = state.db.get().unwrap();
    db.execute(
        "UPDATE users SET last_query_at = datetime('now', 'localtime') WHERE id = ?1",
        [user.id],
    )?;
    Ok(())
}

/// 检查是否达到通知阈值
fn check_notify_threshold(user: &User, diff: &std::collections::HashMap<String, FlowDiff>) -> bool {
    if user.notify_threshold <= 0 {
        return true;
    }

    let uused = diff
        .get("所有通用")
        .map(|d| d.uused)
        .unwrap_or(0.0);

    uused >= user.notify_threshold as f64
}
