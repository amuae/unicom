use rusqlite::{Connection, Result as SqlResult};

pub fn create_tables(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            mobile VARCHAR(11) NOT NULL UNIQUE,
            nickname VARCHAR(50) DEFAULT '',
            query_password VARCHAR(255) NOT NULL,
            login_password_hash VARCHAR(255) DEFAULT '',
            auth_type VARCHAR(20) DEFAULT 'password',
            appid VARCHAR(100) DEFAULT '',
            token_online VARCHAR(255) DEFAULT '',
            cookie TEXT DEFAULT '',
            cookie_created_at DATETIME,
            status VARCHAR(20) DEFAULT 'active',
            last_query_at DATETIME,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            today_query_data TEXT DEFAULT '',
            last_query_data TEXT DEFAULT '',
            last_query_time DATETIME,
            token VARCHAR(24) DEFAULT '',
            notify_enabled INTEGER DEFAULT 0,
            notify_type VARCHAR(50) DEFAULT '',
            notify_params TEXT DEFAULT '',
            notify_title VARCHAR(200) DEFAULT '联通流量提醒',
            notify_subtitle VARCHAR(200) DEFAULT '',
            notify_content TEXT DEFAULT '',
            notify_threshold INTEGER DEFAULT 5120,
            query_interval INTEGER DEFAULT 30,
            last_notify_time TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_users_mobile ON users(mobile);
        CREATE INDEX IF NOT EXISTS idx_users_status ON users(status);
        CREATE INDEX IF NOT EXISTS idx_users_token ON users(token);

        CREATE TABLE IF NOT EXISTS admins (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username VARCHAR(50) NOT NULL UNIQUE,
            password VARCHAR(255) NOT NULL,
            real_name VARCHAR(50) DEFAULT '',
            email VARCHAR(100) DEFAULT '',
            last_login_at DATETIME,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_admins_username ON admins(username);

        CREATE TABLE IF NOT EXISTS admin_sessions (
            token VARCHAR(64) PRIMARY KEY,
            admin_id INTEGER NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            expires_at DATETIME NOT NULL,
            FOREIGN KEY (admin_id) REFERENCES admins(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_admin_sessions_token ON admin_sessions(token);
        CREATE INDEX IF NOT EXISTS idx_admin_sessions_expires ON admin_sessions(expires_at);

        CREATE TABLE IF NOT EXISTS query_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            mobile VARCHAR(11) NOT NULL,
            query_result TEXT,
            query_status VARCHAR(20) DEFAULT 'success',
            error_message TEXT,
            ip_address VARCHAR(45),
            user_agent TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_query_logs_user_id ON query_logs(user_id);
        CREATE INDEX IF NOT EXISTS idx_query_logs_created_at ON query_logs(created_at);

        CREATE TABLE IF NOT EXISTS system_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            log_type VARCHAR(20) NOT NULL,
            log_level VARCHAR(20) DEFAULT 'info',
            message TEXT NOT NULL,
            context TEXT,
            user_id INTEGER,
            ip_address VARCHAR(45),
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_system_logs_type ON system_logs(log_type);
        CREATE INDEX IF NOT EXISTS idx_system_logs_created_at ON system_logs(created_at);

        CREATE TABLE IF NOT EXISTS system_config (
            key VARCHAR(100) PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        INSERT OR IGNORE INTO system_config (key, value) VALUES ('guest_register_enabled', '0');
        INSERT OR IGNORE INTO system_config (key, value) VALUES ('log_level', 'info');
        INSERT OR IGNORE INTO system_config (key, value) VALUES ('log_retention_days', '7');

        CREATE TABLE IF NOT EXISTS user_cron_tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL UNIQUE,
            mobile VARCHAR(11) NOT NULL,
            cron_expression VARCHAR(100) NOT NULL,
            status VARCHAR(20) DEFAULT 'active',
            last_run_at DATETIME,
            last_run_status VARCHAR(20),
            last_run_message TEXT,
            total_runs INTEGER DEFAULT 0,
            success_runs INTEGER DEFAULT 0,
            failed_runs INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_user_cron_tasks_user_id ON user_cron_tasks(user_id);
        CREATE INDEX IF NOT EXISTS idx_user_cron_tasks_status ON user_cron_tasks(status);
        "
    )?;

    // 迁移：为已有 users 表添加 login_password_hash 列（如果不存在）
    migrate_add_column(conn, "users", "login_password_hash", "VARCHAR(255) DEFAULT ''")?;

    // 清理过期 admin sessions
    let _ = conn.execute(
        "DELETE FROM admin_sessions WHERE expires_at < datetime('now')",
        [],
    );

    Ok(())
}

/// 安全地添加列（如果不存在）
fn migrate_add_column(conn: &Connection, table: &str, column: &str, col_type: &str) -> SqlResult<()> {
    let sql = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, col_type);
    // 如果列已存在，ALTER TABLE 会报错，忽略即可
    match conn.execute_batch(&sql) {
        Ok(_) => Ok(()),
        Err(_) => Ok(()), // 列已存在，忽略
    }
}
