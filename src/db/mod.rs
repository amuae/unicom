pub mod schema;

use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

pub type DbPool = r2d2::Pool<SqliteConnectionManager>;

pub fn create_pool(path: &str) -> DbPool {
    let manager = SqliteConnectionManager::file(path);
    r2d2::Pool::builder()
        .max_size(10)
        .connection_customizer(Box::new(ConnectionCustomizer))
        .build(manager)
        .expect("Failed to create database pool")
}

#[derive(Debug)]
struct ConnectionCustomizer;

impl r2d2::CustomizeConnection<Connection, rusqlite::Error> for ConnectionCustomizer {
    fn on_acquire(&self, conn: &mut Connection) -> Result<(), rusqlite::Error> {
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
        Ok(())
    }
}

/// 初始化数据库表（仅需在启动时调用一次）
pub fn init_schema(pool: &DbPool) {
    let conn = pool.get().expect("Failed to get connection for schema init");
    schema::create_tables(&conn).expect("Failed to create tables");
}
