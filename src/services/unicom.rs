use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use anyhow::{Result, Context};
use tracing::{info, warn, error};

use crate::config::UnicomConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResult {
    pub cookie: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowData {
    #[serde(rename = "packageName")]
    pub package_name: Option<String>,
    #[serde(rename = "shareData")]
    pub share_data: Option<serde_json::Value>,
    pub unshared: Option<serde_json::Value>,
    pub resources: Option<serde_json::Value>,
    #[serde(rename = "MlResources")]
    pub ml_resources: Option<serde_json::Value>,
    // 其他字段...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceData {
    pub balance: String,
    pub real_fee: String,
    pub can_use_fee: String,
}

pub struct UnicomService {
    client: Client,
    config: UnicomConfig,
}

impl UnicomService {
    pub fn new(config: UnicomConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .connect_timeout(Duration::from_secs(config.connect_timeout))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, config })
    }

    /// 登录获取 Cookie
    pub async fn login(&self, app_id: &str, token_online: &str) -> Result<LoginResult> {
        info!("开始登录，appId: {}..., login_url: {}", &app_id[..10.min(app_id.len())], &self.config.login_url);

        let params = [
            ("isFirstInstall", "1"),
            ("reqtime", &format!("{}", chrono::Utc::now().timestamp_millis())),
            ("deviceOS", "android15"),
            ("netWay", "Wifi"),
            ("version", "android@12.0500"),
            ("pushPlatform", "OPPO"),
            ("token_online", token_online),
            ("provinceChanel", "general"),
            ("appId", app_id),
            ("simOperator", "5,中国联通,460,01,cn@5,--,460,01,cn"),
            ("deviceModel", "PKB110"),
            ("step", "bindlist"),
            ("deviceBrand", "OPPO"),
            ("flushkey", "1"),
        ];

        let response = self.client
            .post(&self.config.login_url)
            .header("User-Agent", &self.config.user_agent)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                error!("登录请求底层错误: {:?}", e);
                anyhow::anyhow!("登录请求失败: {}", e)
            })?;

        if !response.status().is_success() {
            anyhow::bail!("登录请求失败，HTTP状态码: {}", response.status());
        }

        // 提取 Cookie
        let cookies: Vec<String> = response.headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(|s| s.split(';').next().unwrap_or("").to_string())
            .collect();

        if cookies.is_empty() {
            anyhow::bail!("登录失败：未获取到Cookie");
        }

        let cookie = cookies.join("; ");
        info!("登录成功，Cookie长度: {}", cookie.len());

        Ok(LoginResult {
            cookie,
            created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }

    /// 查询流量数据
    pub async fn query_flow(&self, cookie: &str) -> Result<FlowData> {
        let response = self.client
            .post(&self.config.query_url)
            .header("Cookie", cookie)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", &self.config.user_agent)
            .send()
            .await
            .context("查询流量请求失败")?;

        if !response.status().is_success() {
            anyhow::bail!("查询流量失败，HTTP状态码: {}", response.status());
        }

        let text = response.text().await.context("读取响应失败")?;
        
        // 检查是否是错误响应
        if text.trim().parse::<i64>().is_ok() {
            anyhow::bail!("Cookie已失效");
        }

        let data: serde_json::Value = serde_json::from_str(&text)
            .context("解析流量数据失败")?;

        // 检查错误码
        if let Some(code) = data.get("code").and_then(|v| v.as_str()) {
            if code != "0000" {
                anyhow::bail!("Cookie已失效");
            }
        }

        // 检查是否有有效数据
        if data.get("packageName").is_none() && data.get("shareData").is_none() && data.get("resources").is_none() {
            anyhow::bail!("Cookie已失效");
        }

        serde_json::from_value(data).context("解析流量数据失败")
    }

    /// 查询余额数据
    pub async fn query_balance(&self, cookie: &str) -> Result<BalanceData> {
        let response = self.client
            .post(&self.config.balance_url)
            .header("Cookie", cookie)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", &self.config.user_agent)
            .send()
            .await
            .context("查询余额请求失败")?;

        if !response.status().is_success() {
            anyhow::bail!("查询余额失败，HTTP状态码: {}", response.status());
        }

        let data: serde_json::Value = response.json().await.context("解析余额数据失败")?;

        // 检查错误码
        if let Some(code) = data.get("code").and_then(|v| v.as_str()) {
            if code != "0000" {
                let msg = data.get("msg").and_then(|v| v.as_str()).unwrap_or("Cookie已失效");
                anyhow::bail!("余额查询失败: {}", msg);
            }
        }

        Ok(BalanceData {
            balance: data.get("curntbalancecust").and_then(|v| v.as_str()).unwrap_or("0.00").to_string(),
            real_fee: data.get("realfeecust").and_then(|v| v.as_str()).unwrap_or("0.00").to_string(),
            can_use_fee: data.get("canusefeecust").and_then(|v| v.as_str()).unwrap_or("0.00").to_string(),
        })
    }

    /// 检查 Cookie 是否需要刷新
    pub fn should_refresh_cookie(&self, cookie_created_at: &str) -> bool {
        if cookie_created_at.is_empty() {
            return true;
        }

        if let Ok(created_at) = chrono::NaiveDateTime::parse_from_str(cookie_created_at, "%Y-%m-%d %H:%M:%S") {
            let now = chrono::Local::now().naive_local();
            let age = (now - created_at).num_seconds();
            let remaining = self.config.cookie_ttl as i64 - age;
            
            if remaining < 600 { // 10 分钟
                warn!("Cookie 即将过期，剩余 {} 秒", remaining);
                return true;
            }
        }

        false
    }
}
