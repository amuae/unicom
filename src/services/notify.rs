use std::collections::HashMap;
use anyhow::{Result, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn, error};

use crate::models::user::User;
use crate::services::flow_analysis::{self, FlowAnalysis, FlowDiff};

/// Telegram 通知参数（从用户 notify_params JSON 解析）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyParams {
    pub bot_token: String,
    pub chat_id: String,
    #[serde(default = "default_api_host")]
    pub api_host: String,
}

fn default_api_host() -> String {
    "https://api.telegram.org".to_string()
}

/// 通知模块复用 flow_analysis 的 format_flow
pub use crate::services::flow_analysis::format_flow;

/// 解析用户的 notify_params JSON
pub fn parse_notify_params(params_json: &str) -> Result<NotifyParams> {
    serde_json::from_str(params_json)
        .with_context(|| format!("解析 notify_params 失败: {}", params_json))
}

/// 发送 Telegram 消息
pub async fn send_telegram(params: &NotifyParams, title: &str, content: &str) -> Result<()> {
    if params.bot_token.is_empty() || params.chat_id.is_empty() {
        anyhow::bail!("Telegram 配置不完整：bot_token 或 chat_id 为空");
    }

    let api_host = if params.api_host.trim().is_empty() {
        "https://api.telegram.org"
    } else {
        params.api_host.trim_end_matches('/')
    };
    let url = format!("{}/bot{}/sendMessage", api_host, params.bot_token);

    let message = format!("*{}*\n\n{}", title, content);

    let body = serde_json::json!({
        "chat_id": params.chat_id,
        "text": message,
        "parse_mode": "Markdown",
        "disable_web_page_preview": true
    });

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("创建 HTTP 客户端失败")?;

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .context("发送 Telegram 请求失败")?;

    let status = resp.status();
    let resp_body: serde_json::Value = resp.json().await.unwrap_or_default();

    if status.is_success() && resp_body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        info!("Telegram 发送通知消息成功🎉");
        Ok(())
    } else {
        let desc = resp_body
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("未知错误");
        anyhow::bail!("Telegram 发送失败: {} (HTTP {})", desc, status)
    }
}

/// 构建通知内容：替换模板中的占位符
///
/// 占位符：[套餐] [时间] [日期] [时长]
///         [桶名.总量] [桶名.已用] [桶名.剩余] [桶名.用量] [桶名.今日用量]
/// 桶名：所有通用/所有免流/所有流量/通用有限/通用不限/区域有限/区域不限/免流有限/免流不限
///
/// - 绝对值（总量/已用/剩余）来自当前查询的 buckets
/// - 用量（diff）来自 notify_diff（上次通知基准）
/// - 今日用量（diff）来自 daily_diff（今日首次查询基准）
pub fn build_notify_content(
    template: &str,
    data: &FlowAnalysis,
    notify_diff: &HashMap<String, FlowDiff>,
    daily_diff: &HashMap<String, FlowDiff>,
    interval: &str,
) -> String {
    let mut result = template.to_string();

    // 基础占位符
    result = result.replace("[套餐]", &data.main_package);
    result = result.replace("[时间]", &chrono::Local::now().format("%H:%M:%S").to_string());
    result = result.replace("[日期]", &chrono::Local::now().format("%Y-%m-%d").to_string());
    result = result.replace("[时长]", interval);

    // 桶级占位符：中文显示名 → flow_analysis 中的 key
    let bucket_mappings: &[(&str, &str)] = &[
        ("所有通用", "所有通用"),
        ("所有免流", "所有免流"),
        ("所有流量", "所有流量"),
        ("通用有限", "common_limited"),
        ("通用不限", "common_unlimited"),
        ("区域有限", "regional_limited"),
        ("区域不限", "regional_unlimited"),
        ("免流有限", "targeted_limited"),
        ("免流不限", "targeted_unlimited"),
    ];

    for &(display_name, key) in bucket_mappings {
        let bucket = data.buckets.get(key);
        let notify_entry = notify_diff.get(key);
        let daily_entry = daily_diff.get(key);

        let total = bucket.map(|b| b.total).unwrap_or(0.0);
        let used = bucket.map(|b| b.used).unwrap_or(0.0);
        let remain = bucket.map(|b| b.remain).unwrap_or(0.0);
        let uused = notify_entry.map(|d| d.uused).unwrap_or(0.0);
        let today = daily_entry.map(|d| d.today).unwrap_or(0.0);

        result = result.replace(&format!("[{}.总量]", display_name), &format_flow(total));
        result = result.replace(&format!("[{}.已用]", display_name), &format_flow(used));
        result = result.replace(&format!("[{}.剩余]", display_name), &format_flow(remain));
        result = result.replace(&format!("[{}.用量]", display_name), &format_flow(uused));
        result = result.replace(&format!("[{}.今日用量]", display_name), &format_flow(today));
    }

    // 如果模板为空或未匹配到占位符，使用默认模板
    if template.is_empty() || template == result {
        let mut lines = vec![
            format!("套餐: {}", data.main_package),
            format!("查询时间: {}", data.timestamp),
        ];
        for (key, bucket) in &data.buckets {
            let notify_entry = notify_diff.get(key);
            let daily_entry = daily_diff.get(key);
            let uused = notify_entry.map(|d| d.uused).unwrap_or(0.0);
            let today = daily_entry.map(|d| d.today).unwrap_or(0.0);
            let display = flow_analysis::bucket_display_name(key);
            lines.push(format!(
                "{}: 总量 {} / 已用 {} / 剩余 {} / 用量 {} / 今日 {}",
                display,
                format_flow(bucket.total),
                format_flow(bucket.used),
                format_flow(bucket.remain),
                format_flow(uused),
                format_flow(today),
            ));
        }
        return lines.join("\n");
    }

    result
}

/// 发送流量通知
pub async fn send_flow_notify(
    user: &User,
    data: &FlowAnalysis,
    notify_diff: &HashMap<String, FlowDiff>,
    daily_diff: &HashMap<String, FlowDiff>,
    interval: &str,
) -> Result<()> {
    if user.notify_enabled == 0 {
        return Ok(());
    }

    if user.notify_type != "telegram" {
        warn!("暂不支持的通知类型: {}", user.notify_type);
        return Ok(());
    }

    let params = match parse_notify_params(&user.notify_params) {
        Ok(p) => p,
        Err(e) => {
            error!("用户 {} 解析通知参数失败: {}", user.id, e);
            return Ok(());
        }
    };

    let title = if user.notify_title.is_empty() {
        "📊 流量查询通知".to_string()
    } else {
        // 标题也替换占位符
        build_notify_content(&user.notify_title, data, notify_diff, daily_diff, interval)
    };

    let content = build_notify_content(
        &user.notify_content,
        data,
        notify_diff,
        daily_diff,
        interval,
    );

    if let Err(e) = send_telegram(&params, &title, &content).await {
        error!("用户 {} 发送通知失败: {}", user.id, e);
        // 通知失败不影响主流程
    }

    Ok(())
}

/// 发送凭证失效通知
pub async fn send_credential_expired_notify(user: &User, error: &str) -> Result<()> {
    if user.notify_enabled == 0 || user.notify_type != "telegram" {
        return Ok(());
    }

    let params = match parse_notify_params(&user.notify_params) {
        Ok(p) => p,
        Err(e) => {
            error!("用户 {} 解析通知参数失败: {}", user.id, e);
            return Ok(());
        }
    };

    let title = "⚠️ 凭证失效通知";
    let content = format!(
        "用户: {}\n手机号: {}\n错误: {}\n时间: {}\n\n请尽快更新凭证。",
        user.nickname,
        user.mobile,
        error,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
    );

    if let Err(e) = send_telegram(&params, title, &content).await {
        error!("用户 {} 发送凭证失效通知失败: {}", user.id, e);
    }

    Ok(())
}

/// 发送测试通知
pub async fn send_test_notify(user: &User) -> Result<()> {
    if user.notify_type != "telegram" {
        anyhow::bail!("暂不支持的通知类型: {}", user.notify_type);
    }

    let params = parse_notify_params(&user.notify_params)?;

    let title = "🔔 通知测试";
    let content = format!(
        "这是一条测试通知\n用户: {}\n时间: {}",
        user.mobile,
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
    );

    send_telegram(&params, title, &content).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_flow() {
        assert_eq!(format_flow(512.0), "512M");
        assert_eq!(format_flow(1024.0), "1G");
        assert_eq!(format_flow(1048576.0), "1T");
        assert_eq!(format_flow(-1.0), "0");
    }

    #[test]
    fn test_parse_notify_params() {
        let json = r#"{"bot_token":"abc","chat_id":"123"}"#;
        let params = parse_notify_params(json).unwrap();
        assert_eq!(params.bot_token, "abc");
        assert_eq!(params.chat_id, "123");
        assert_eq!(params.api_host, "https://api.telegram.org");
    }
}
