use serde::Deserialize;
use config::{ConfigError, Config as ConfigRs, File};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_database_path")]
    pub database_path: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub unicom: UnicomConfig,
    #[serde(default)]
    pub notify: NotifyConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UnicomConfig {
    #[serde(default = "default_login_url")]
    pub login_url: String,
    #[serde(default = "default_query_url")]
    pub query_url: String,
    #[serde(default = "default_balance_url")]
    pub balance_url: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    #[serde(default = "default_cookie_ttl")]
    pub cookie_ttl: u64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NotifyConfig {
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
}

fn default_database_path() -> String { "unicom.db".to_string() }
fn default_log_level() -> String { "info".to_string() }
fn default_login_url() -> String { "https://m.client.10010.com/mobileService/onLine.htm".to_string() }
fn default_query_url() -> String { "https://m.client.10010.com/servicequerybusiness/operationservice/queryOcsPackageFlowLeftContentRevisedInJune".to_string() }
fn default_balance_url() -> String { "https://m.client.10010.com/servicequerybusiness/balancenew/accountBalancenew.htm".to_string() }
fn default_timeout() -> u64 { 30 }
fn default_connect_timeout() -> u64 { 10 }
fn default_user_agent() -> String { "Mozilla/5.0 (iPhone; CPU iPhone OS 15_0 like Mac OS X) AppleWebKit/605.1.15".to_string() }
fn default_cookie_ttl() -> u64 { 7200 }

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let config = ConfigRs::builder()
            .add_source(File::with_name("config"))
            .build()?;

        let mut cfg: Config = config.try_deserialize()?;
        
        // config crate 不会正确应用嵌套结构体的 serde 默认值
        // 手动填充空字段
        if cfg.unicom.login_url.is_empty() {
            cfg.unicom.login_url = default_login_url();
        }
        if cfg.unicom.query_url.is_empty() {
            cfg.unicom.query_url = default_query_url();
        }
        if cfg.unicom.balance_url.is_empty() {
            cfg.unicom.balance_url = default_balance_url();
        }
        if cfg.unicom.timeout == 0 {
            cfg.unicom.timeout = default_timeout();
        }
        if cfg.unicom.connect_timeout == 0 {
            cfg.unicom.connect_timeout = default_connect_timeout();
        }
        if cfg.unicom.user_agent.is_empty() {
            cfg.unicom.user_agent = default_user_agent();
        }
        if cfg.unicom.cookie_ttl == 0 {
            cfg.unicom.cookie_ttl = default_cookie_ttl();
        }
        
        Ok(cfg)
    }
}
