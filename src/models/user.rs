use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub mobile: String,
    pub nickname: String,
    pub query_password: String,
    pub auth_type: String,
    pub appid: String,
    pub token_online: String,
    pub cookie: String,
    pub cookie_created_at: Option<NaiveDateTime>,
    pub status: String,
    pub last_query_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub today_query_data: String,
    pub last_query_data: String,
    pub last_query_time: Option<NaiveDateTime>,
    pub token: String,
    pub notify_enabled: i32,
    pub notify_type: String,
    pub notify_params: String,
    pub notify_title: String,
    pub notify_subtitle: String,
    pub notify_content: String,
    pub notify_threshold: i32,
    pub query_interval: i32,
    pub last_notify_time: Option<String>,
}

