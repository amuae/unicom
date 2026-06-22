use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub mobile: String,
    pub nickname: String,
    pub query_password: String,
    pub login_password_hash: String,
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

/// 完整的 SELECT 列列表（保持字段顺序与 User 结构体一致）
pub const USER_COLUMNS: &str = "\
    id, mobile, nickname, query_password, \
    COALESCE(login_password_hash, ''), \
    auth_type, appid, token_online, cookie, cookie_created_at, \
    status, last_query_at, created_at, updated_at, \
    COALESCE(today_query_data, ''), COALESCE(last_query_data, ''), last_query_time, \
    token, notify_enabled, notify_type, notify_params, notify_title, notify_subtitle, \
    notify_content, notify_threshold, query_interval, last_notify_time";

impl User {
    /// 从数据库行构造 User（统一的行映射，消除重复）
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(User {
            id: row.get(0)?,
            mobile: row.get(1)?,
            nickname: row.get(2)?,
            query_password: row.get(3)?,
            login_password_hash: row.get(4)?,
            auth_type: row.get(5)?,
            appid: row.get(6)?,
            token_online: row.get(7)?,
            cookie: row.get(8)?,
            cookie_created_at: row.get(9)?,
            status: row.get(10)?,
            last_query_at: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
            today_query_data: row.get(14)?,
            last_query_data: row.get(15)?,
            last_query_time: row.get(16)?,
            token: row.get(17)?,
            notify_enabled: row.get(18)?,
            notify_type: row.get(19)?,
            notify_params: row.get(20)?,
            notify_title: row.get(21)?,
            notify_subtitle: row.get(22)?,
            notify_content: row.get(23)?,
            notify_threshold: row.get(24)?,
            query_interval: row.get(25)?,
            last_notify_time: row.get(26)?,
        })
    }
}
