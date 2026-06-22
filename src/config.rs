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

#[derive(Debug, Clone, Deserialize)]
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

impl Default for UnicomConfig {
    fn default() -> Self {
        Self {
            login_url: default_login_url(),
            query_url: default_query_url(),
            balance_url: default_balance_url(),
            timeout: default_timeout(),
            connect_timeout: default_connect_timeout(),
            user_agent: default_user_agent(),
            cookie_ttl: default_cookie_ttl(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct NotifyConfig {
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    #[serde(default)]
    pub allowed_origins: Vec<String>,
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

        let cfg: Config = config.try_deserialize()?;
        Ok(cfg)
    }
}
