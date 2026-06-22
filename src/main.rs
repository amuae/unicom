use actix_cors::Cors;
use actix_web::{web, App, HttpServer, middleware, HttpRequest, HttpResponse};

use tracing_subscriber::EnvFilter;
use std::sync::Arc;
use std::io::{self, Write, IsTerminal};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

mod config;
mod db;
mod models;
mod services;
mod handlers;
mod cron;
mod static_files;

use static_files::StaticFile;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub struct AppState {
    pub db: db::DbPool,
    pub config: config::Config,
    pub static_files: Arc<HashMap<String, StaticFile>>,
    pub rate_limiter: Arc<Mutex<RateLimiter>>,
    pub session_cache: Arc<Mutex<handlers::auth::SessionCache>>,
}

/// 简单的内存速率限制器（IP → 最近请求记录）
pub struct RateLimiter {
    /// IP → (请求次数, 窗口开始时间)
    attempts: HashMap<String, (u32, Instant)>,
    /// 最大请求数（每个窗口）
    max_requests: u32,
    /// 窗口时长（秒）
    window_secs: u64,
}

impl RateLimiter {
    fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            attempts: HashMap::new(),
            max_requests,
            window_secs,
        }
    }

    /// 检查是否超过速率限制，返回 true 表示允许，false 表示拒绝
    fn check(&mut self, key: &str) -> bool {
        let now = Instant::now();
        let entry = self.attempts.entry(key.to_string()).or_insert((0, now));

        // 窗口过期，重置
        if now.duration_since(entry.1).as_secs() >= self.window_secs {
            *entry = (1, now);
            return true;
        }

        entry.0 += 1;
        entry.0 <= self.max_requests
    }

    /// 清理过期条目
    pub fn cleanup(&mut self) {
        let now = Instant::now();
        self.attempts.retain(|_, (_, start)| {
            now.duration_since(*start).as_secs() < self.window_secs * 2
        });
    }
}

// ═══════════════════════════════════════════════
// 工具函数
// ═══════════════════════════════════════════════

/// 读取一行输入，EOF 时返回空字符串
fn read_line() -> String {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) | Err(_) => String::new(), // EOF 或错误
        Ok(_) => input.trim().to_string(),
    }
}

fn get_pid() -> Option<u32> {
    let pid_file = format!("{}/unicom.pid", get_install_dir());
    if let Ok(content) = std::fs::read_to_string(&pid_file) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            #[cfg(target_os = "windows")]
            {
                let output = std::process::Command::new("tasklist")
                    .args(["/FI", &format!("PID eq {}", pid), "/NH"])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .output();
                if let Ok(out) = output {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    if stdout.contains(&pid.to_string()) {
                        return Some(pid);
                    }
                }
                return None;
            }
            #[cfg(not(target_os = "windows"))]
            {
                if std::process::Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .is_ok()
                {
                    return Some(pid);
                }
            }
        }
    }
    None
}

fn is_running() -> bool {
    get_pid().is_some()
}

fn get_install_dir() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_string_lossy().to_string()))
        .unwrap_or_else(|| ".".to_string())
}

/// 清屏
fn clear_screen() {
    print!("\x1B[2J\x1B[H");
    io::stdout().flush().ok();
}

// ═══════════════════════════════════════════════
// 快捷指令
// ═══════════════════════════════════════════════

fn cmd_help() {
    println!("Unicom 联通流量查询系统 v{}", VERSION);
    println!();
    println!("用法: unicom [命令]");
    println!();
    println!("命令:");
    println!("  start       启动服务");
    println!("  stop        停止服务");
    println!("  restart     重启服务");
    println!("  status      查看状态");
    println!("  reset-pass  重置管理员账密");
    println!("  version     显示版本");
    println!("  help        显示帮助");
    println!("  menu        进入交互菜单");
    println!();
    println!("无参数则进入交互菜单");
}

fn cmd_version() {
    println!("unicom {}", VERSION);
}

fn cmd_status() {
    let config = match config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            println!("❌ 加载配置失败: {}", e);
            return;
        }
    };
    println!("Unicom v{}", VERSION);
    println!("  地址: {}:{}", config.host, config.port);
    println!("  数据库: {}", config.database_path);
    match get_pid() {
        Some(pid) => {
            println!("  状态: ✅ 运行中 (PID: {})", pid);
            
            #[cfg(not(target_os = "windows"))]
            {
            let status_path = format!("/proc/{}/status", pid);
            let stat_path = format!("/proc/{}/stat", pid);
            
            // 内存信息
            if let Ok(content) = std::fs::read_to_string(&status_path) {
                for line in content.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(kb) = line.split_whitespace().nth(1) {
                            let mb: f64 = kb.parse::<f64>().unwrap_or(0.0) / 1024.0;
                            println!("  内存: {:.1} MB", mb);
                        }
                    }
                    if line.starts_with("Threads:") {
                        if let Some(threads) = line.split_whitespace().nth(1) {
                            println!("  线程: {}", threads);
                        }
                    }
                }
            }
            
            // CPU 占用率（采样 500ms）
            print!("  CPU: 采样中...");
            io::stdout().flush().ok();
            if let (Ok(content1), Ok(sys1)) = (
                std::fs::read_to_string(&stat_path),
                std::fs::read_to_string("/proc/stat"),
            ) {
                let fields1: Vec<&str> = content1.split_whitespace().collect();
                let cpu_line1: Vec<u64> = sys1.lines().next().unwrap_or("")
                    .split_whitespace().skip(1)
                    .filter_map(|s| s.parse().ok())
                    .collect();
                let proc_cpu1: u64 = if fields1.len() > 15 {
                    fields1[13].parse::<u64>().unwrap_or(0) + fields1[14].parse::<u64>().unwrap_or(0)
                } else { 0 };
                let sys_cpu1: u64 = cpu_line1.iter().sum();

                std::thread::sleep(std::time::Duration::from_millis(500));

                if let (Ok(content2), Ok(sys2)) = (
                    std::fs::read_to_string(&stat_path),
                    std::fs::read_to_string("/proc/stat"),
                ) {
                    let fields2: Vec<&str> = content2.split_whitespace().collect();
                    let cpu_line2: Vec<u64> = sys2.lines().next().unwrap_or("")
                        .split_whitespace().skip(1)
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    let proc_cpu2: u64 = if fields2.len() > 15 {
                        fields2[13].parse::<u64>().unwrap_or(0) + fields2[14].parse::<u64>().unwrap_or(0)
                    } else { 0 };
                    let sys_cpu2: u64 = cpu_line2.iter().sum();

                    let proc_delta = proc_cpu2.saturating_sub(proc_cpu1) as f64;
                    let sys_delta = sys_cpu2.saturating_sub(sys_cpu1) as f64;

                    if sys_delta > 0.0 {
                        let cpu_pct = proc_delta / sys_delta * 100.0;
                        print!("\r  CPU: {:.1}%    \n", cpu_pct);
                    } else {
                        print!("\r  CPU: 0.0%    \n");
                    }
                }
            }
            
            // 运行时间
            if let Ok(content) = std::fs::read_to_string(&stat_path) {
                let fields: Vec<&str> = content.split_whitespace().collect();
                if fields.len() > 21 {
                    let starttime: u64 = fields[21].parse().unwrap_or(0);
                    if let Ok(uptime_content) = std::fs::read_to_string("/proc/uptime") {
                        let uptime_secs: f64 = uptime_content
                            .split_whitespace()
                            .next()
                            .unwrap_or("0")
                            .parse()
                            .unwrap_or(0.0);
                        let hz = 100.0;
                        let start_secs = starttime as f64 / hz;
                        let run_secs = uptime_secs - start_secs;
                        if run_secs > 0.0 {
                            let days = (run_secs / 86400.0) as u64;
                            let hours = ((run_secs % 86400.0) / 3600.0) as u64;
                            let minutes = ((run_secs % 3600.0) / 60.0) as u64;
                            if days > 0 {
                                println!("  运行时间: {}天 {}小时 {}分钟", days, hours, minutes);
                            } else if hours > 0 {
                                println!("  运行时间: {}小时 {}分钟", hours, minutes);
                            } else {
                                println!("  运行时间: {}分钟", minutes);
                            }
                        }
                    }
                }
            }
            } // end #[cfg(not(target_os = "windows"))]
        }
        None => println!("  状态: ❌ 未运行"),
    }
}

fn cmd_start() {
    if is_running() {
        println!("服务已在运行中 (PID: {})", get_pid().unwrap_or(0));
        return;
    }

    let android_script = "/data/adb/service.d/unicom.sh";
    if std::path::Path::new(android_script).exists() {
        println!("启动服务...");
        std::process::Command::new("sh")
            .args([android_script, "start"])
            .status()
            .ok();
        std::thread::sleep(std::time::Duration::from_secs(3));
        if is_running() {
            println!("✅ 启动成功 (PID: {})", get_pid().unwrap_or(0));
        } else {
            println!("❌ 启动失败，请查看日志");
        }
        return;
    }

    println!("启动服务...");
    let install_dir = get_install_dir();
    #[cfg(target_os = "windows")]
    {
        let exe = format!("{}/unicom.exe", install_dir);
        use std::os::windows::process::CommandExt;
        let result = std::process::Command::new(&exe)
            .current_dir(&install_dir)
            .creation_flags(0x08000000)
            .stdin(std::process::Stdio::null())
            .spawn();
        match result {
            Ok(child) => {
                std::fs::write(format!("{}/unicom.pid", install_dir), child.id().to_string()).ok();
                std::thread::sleep(std::time::Duration::from_secs(2));
                if is_running() {
                    println!("✅ 启动成功 (PID: {})", child.id());
                } else {
                    println!("❌ 启动失败，请查看日志");
                }
            }
            Err(e) => {
                println!("❌ 启动失败: {}", e);
                println!("  确保 unicom.exe 在 {} 目录下", install_dir);
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let exe = format!("{}/unicom", install_dir);
        match std::process::Command::new(&exe)
            .current_dir(&install_dir)
            .spawn()
        {
            Ok(_) => {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if is_running() {
                    println!("✅ 启动成功 (PID: {})", get_pid().unwrap_or(0));
                } else {
                    println!("❌ 启动失败，请查看日志");
                }
            }
            Err(e) => {
                println!("❌ 启动失败: {}", e);
                println!("  确保 {} 存在且有执行权限", exe);
            }
        }
    }
}

fn cmd_stop() {
    let android_script = "/data/adb/service.d/unicom.sh";
    if std::path::Path::new(android_script).exists() {
        println!("停止服务...");
        std::process::Command::new("sh")
            .args([android_script, "stop"])
            .status()
            .ok();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let pid_file = format!("{}/unicom.pid", get_install_dir());
        std::fs::remove_file(&pid_file).ok();
        if !is_running() {
            println!("✅ 已停止");
        } else {
            println!("❌ 停止失败");
        }
        return;
    }

    match get_pid() {
        Some(pid) => {
            println!("停止服务 (PID: {})...", pid);
            #[cfg(target_os = "windows")]
            {
                std::process::Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/F"])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .ok();
            }
            #[cfg(not(target_os = "windows"))]
            {
                // 先尝试 SIGTERM
                std::process::Command::new("kill")
                    .arg(pid.to_string())
                    .status()
                    .ok();
                std::thread::sleep(std::time::Duration::from_secs(2));
                // 如果还在运行，用 SIGKILL
                if is_running() {
                    println!("SIGTERM 无效，尝试 SIGKILL...");
                    std::process::Command::new("kill")
                        .args(["-9", &pid.to_string()])
                        .status()
                        .ok();
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            if !is_running() {
                let pid_file = format!("{}/unicom.pid", get_install_dir());
                std::fs::remove_file(&pid_file).ok();
                println!("✅ 已停止");
            } else {
                println!("❌ 停止失败");
            }
        }
        None => println!("服务未运行"),
    }
}

fn cmd_restart() {
    cmd_stop();
    std::thread::sleep(std::time::Duration::from_secs(2));
    cmd_start();
}

fn cmd_reset_pass() {
    let config = match config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            println!("❌ 加载配置失败: {}", e);
            return;
        }
    };
    let conn = match rusqlite::Connection::open(&config.database_path) {
        Ok(c) => c,
        Err(e) => {
            println!("❌ 打开数据库失败: {}", e);
            return;
        }
    };
    conn.execute_batch("PRAGMA foreign_keys = ON;").ok();

    // 使用密码学安全随机数生成用户名和密码
    use rand::Rng;
    let mut rng = rand::rngs::OsRng;
    let chars: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let username: String = (0..8).map(|_| {
        let idx = rng.gen_range(0..chars.len());
        chars[idx] as char
    }).collect();
    let password: String = (0..16).map(|_| {
        let idx = rng.gen_range(0..chars.len());
        chars[idx] as char
    }).collect();
    let username = format!("admin_{}", username);

    let hashed = match bcrypt::hash(&password, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => {
            println!("❌ 密码哈希失败: {}", e);
            return;
        }
    };

    let admin_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM admins", [], |row| row.get(0))
        .unwrap_or(0);

    let result = if admin_count > 0 {
        conn.execute(
            "UPDATE admins SET username = ?1, password = ?2 WHERE id = (SELECT id FROM admins LIMIT 1)",
            rusqlite::params![username, hashed],
        )
    } else {
        conn.execute(
            "INSERT INTO admins (username, password) VALUES (?1, ?2)",
            rusqlite::params![username, hashed],
        )
    };

    match result {
        Ok(rows) if rows > 0 => {
            println!("✅ 管理员账密已重置！");
            println!("   用户名: {}", username);
            println!("   密码: {}", password);
            println!("\n⚠️  请立即登录并修改密码！");
        }
        Ok(_) => {
            println!("❌ 重置失败：未找到管理员账号");
        }
        Err(e) => {
            println!("❌ 重置失败: {}", e);
        }
    }
}

// ═══════════════════════════════════════════════
// 交互菜单
// ═══════════════════════════════════════════════

fn show_main_menu() {
    let status = if is_running() {
        format!("运行中 (PID: {})", get_pid().unwrap_or(0))
    } else {
        "未运行".to_string()
    };

    println!();
    println!("┌─────────────────────────────────────┐");
    println!("│      Unicom 联通流量查询系统        │");
    println!("│             v{:<21}│", VERSION);
    println!("├─────────────────────────────────────┤");
    println!("│  状态: {:<28}│", status);
    println!("├─────────────────────────────────────┤");
    println!("│  1. 启动服务                        │");
    println!("│  2. 重启服务                        │");
    println!("│  3. 停止服务                        │");
    println!("│  4. 运行状态                        │");
    println!("│  5. 管理账密                        │");
    println!("│  6. 检查更新                        │");
    println!("│  7. 卸载程序                        │");
    println!("│  0. 退出                            │");
    println!("└─────────────────────────────────────┘");
    print!("请选择: ");
    io::stdout().flush().ok();
}

fn show_admin_menu() {
    println!();
    println!("┌─────────────────────────────────────┐");
    println!("│           管理员账密                │");
    println!("├─────────────────────────────────────┤");
    println!("│  1. 重置管理员账密                  │");
    println!("│  2. 查看管理员列表                  │");
    println!("│  0. 返回                            │");
    println!("└─────────────────────────────────────┘");
    print!("请选择: ");
    io::stdout().flush().ok();
}

fn list_admins() {
    let config = match config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            println!("❌ 加载配置失败: {}", e);
            return;
        }
    };
    let conn = match rusqlite::Connection::open(&config.database_path) {
        Ok(c) => c,
        Err(e) => {
            println!("❌ 打开数据库失败: {}", e);
            return;
        }
    };
    conn.execute_batch("PRAGMA foreign_keys = ON;").ok();

    println!("\n=== 管理员列表 ===");
    let mut stmt = match conn.prepare("SELECT id, username, real_name FROM admins") {
        Ok(s) => s,
        Err(e) => {
            println!("❌ 查询失败: {}", e);
            return;
        }
    };

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    });

    match rows {
        Ok(rows) => {
            let mut count = 0;
            for row in rows {
                match row {
                    Ok((id, username, real_name)) => {
                        let display_name = if real_name.is_empty() { "-" } else { &real_name };
                        println!("  {}. {} ({})", id, username, display_name);
                        count += 1;
                    }
                    Err(e) => {
                        println!("  ⚠️  读取行失败: {}", e);
                    }
                }
            }
            if count == 0 {
                println!("  暂无管理员");
            }
        }
        Err(e) => {
            println!("❌ 查询失败: {}", e);
        }
    }
}

fn do_update() {
    let install_dir = get_install_dir();
    println!("\n=== 更新 Unicom ===");
    println!("安装目录: {}", install_dir);
    println!();
    #[cfg(target_os = "windows")]
    {
        println!("请在 PowerShell 中执行以下命令更新:");
        println!("  irm https://ghfast.top/https://raw.githubusercontent.com/amuae/unicom/main/deploy.ps1 | iex");
        println!("或手动下载: https://github.com/amuae/unicom/releases/latest");
    }
    #[cfg(not(target_os = "windows"))]
    {
        println!("请执行以下命令更新:");
        println!("  curl -fsSL https://ghfast.top/https://raw.githubusercontent.com/amuae/unicom/main/deploy.sh | sh");
        println!();
        print!("是否现在执行？(y/N): ");
        io::stdout().flush().ok();

        if read_line().to_lowercase() == "y" {
            println!("正在更新...");
            match std::process::Command::new("sh")
                .args(["-c", "curl -fsSL https://ghfast.top/https://raw.githubusercontent.com/amuae/unicom/main/deploy.sh | sh"])
                .status()
            {
                Ok(status) if status.success() => println!("✅ 更新完成"),
                Ok(status) => println!("⚠️  更新脚本退出码: {}", status.code().unwrap_or(-1)),
                Err(e) => println!("❌ 执行更新脚本失败: {}", e),
            }
        }
    }
}

fn do_uninstall() {
    let install_dir = get_install_dir();
    println!("\n=== 卸载 Unicom ===");
    println!("安装目录: {}", install_dir);
    println!();
    println!("⚠️  此操作将：");
    println!("  - 停止服务");
    println!("  - 删除所有文件（包括数据库）");
    println!();
    print!("确认卸载？(输入 yes): ");
    io::stdout().flush().ok();

    if read_line() == "yes" {
        cmd_stop();
        println!("删除文件...");
        match std::fs::remove_dir_all(&install_dir) {
            Ok(_) => {}
            Err(e) => println!("⚠️  删除文件失败: {}", e),
        }

        let service_script = "/data/adb/service.d/unicom.sh";
        if std::path::Path::new(service_script).exists() {
            std::fs::remove_file(service_script).ok();
        }

        println!("✅ 卸载完成");
        std::process::exit(0);
    } else {
        println!("已取消");
    }
}

fn handle_admin_menu() {
    loop {
        show_admin_menu();
        match read_line().as_str() {
            "1" => cmd_reset_pass(),
            "2" => list_admins(),
            "0" | "q" => break,
            "" => break, // EOF
            _ => println!("无效选择"),
        }
    }
}

fn interactive_menu() {
    loop {
        clear_screen();
        show_main_menu();
        match read_line().as_str() {
            "1" => cmd_start(),
            "2" => cmd_restart(),
            "3" => cmd_stop(),
            "4" => cmd_status(),
            "5" => handle_admin_menu(),
            "6" => do_update(),
            "7" => do_uninstall(),
            "0" | "q" | "quit" | "exit" => {
                println!("退出");
                std::process::exit(0);
            }
            "" => {
                // EOF — 非交互环境，直接启动服务
                break;
            }
            _ => println!("无效选择"),
        }
        // 操作完成后暂停，等待用户按回车
        print!("\n按回车返回菜单...");
        io::stdout().flush().ok();
        read_line();
    }
}

// ═══════════════════════════════════════════════
// 主入口
// ═══════════════════════════════════════════════

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let install_dir = get_install_dir();
    std::env::set_current_dir(&install_dir).ok();

    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "start" => {
                cmd_start();
                std::process::exit(0);
            }
            "stop" => cmd_stop(),
            "restart" => cmd_restart(),
            "status" => cmd_status(),
            "reset-pass" => cmd_reset_pass(),
            "version" | "-v" | "--version" => cmd_version(),
            "help" | "-h" | "--help" => cmd_help(),
            "menu" => interactive_menu(),
            _ => {
                println!("未知命令: {}", args[1]);
                cmd_help();
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    let has_tty = io::stdin().is_terminal();
    
    if has_tty {
        interactive_menu();
        // interactive_menu 只在用户选 0 时 exit，EOF 时 break 到这里
        // break 后继续启动服务（用户从菜单选 start 后也会到这里）
    }

    // 加载配置
    let config = config::Config::load().expect("Failed to load config");

    // 初始化日志
    let log_level = &config.log_level;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(log_level))
        )
        .init();
    
    // 初始化数据库连接池
    let db = db::create_pool(&config.database_path);
    db::init_schema(&db);
    tracing::info!("Database pool initialized (max_size=10, WAL mode, busy_timeout=5s)");

    // 加载静态文件
    let static_files = Arc::new(static_files::load_static_files());
    tracing::info!("Loaded {} static files", static_files.len());

    // 创建速率限制器（10 次/分钟）
    let rate_limiter = Arc::new(Mutex::new(RateLimiter::new(10, 60)));

    // 创建 session 缓存（30 秒 TTL，避免每次 admin 请求都查 DB）
    let session_cache = Arc::new(Mutex::new(handlers::auth::SessionCache::new(30)));

    // 创建应用状态
    let app_state = AppState {
        db: db.clone(),
        config: config.clone(),
        static_files: static_files.clone(),
        rate_limiter: rate_limiter.clone(),
        session_cache: session_cache.clone(),
    };

    // 启动定时任务
    let cron_state = app_state.clone();
    tokio::spawn(async move {
        cron::start_cron_jobs(cron_state).await;
    });

    // 启动定期清理任务（每 5 分钟清理过期 session + rate limiter）
    let cleanup_rl = rate_limiter.clone();
    let cleanup_sc = session_cache.clone();
    let cleanup_db = db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            {
                let mut rl = cleanup_rl.lock().unwrap();
                rl.cleanup();
            }
            {
                let mut sc = cleanup_sc.lock().unwrap();
                sc.cleanup();
            }
            if let Ok(conn) = cleanup_db.get() {
                let _ = conn.execute(
                    "DELETE FROM admin_sessions WHERE expires_at < datetime('now')",
                    [],
                );
            }
        }
    });

    // 写入 PID 文件
    let pid = std::process::id();
    std::fs::write("unicom.pid", pid.to_string()).ok();

    tracing::info!("Starting server at {}:{}", config.host, config.port);
    println!("Unicom v{} 启动成功 http://{}:{}", VERSION, config.host, config.port);

    // 启动 HTTP 服务器
    let allowed_origins = config.notify.allowed_origins.clone();
    HttpServer::new(move || {
        let cors = if allowed_origins.is_empty() {
            Cors::default()
                .allowed_origin("http://localhost")
                .allowed_origin("http://127.0.0.1")
                .allowed_origin("http://localhost:8080")
                .allowed_origin("http://127.0.0.1:8080")
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                .allowed_headers(vec!["Content-Type", "Authorization"])
                .max_age(3600)
        } else {
            let mut cors_builder = Cors::default();
            for origin in &allowed_origins {
                cors_builder = cors_builder.allowed_origin(origin);
            }
            cors_builder
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                .allowed_headers(vec!["Content-Type", "Authorization"])
                .max_age(3600)
        };

        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(cors)
            .wrap(middleware::Logger::default())
            // API 路由
            .service(handlers::auth::login)
            .service(handlers::auth::register)
            .service(handlers::auth::user_login)
            .service(handlers::auth::user_register)
            // 查询
            .service(handlers::query::query_flow)
            .service(handlers::query::query_balance)
            // 用户配置
            .service(handlers::user::get_user_config)
            .service(handlers::user::update_user_config)
            // 管理后台
            .service(handlers::admin::get_users)
            .service(handlers::admin::get_user_detail)
            .service(handlers::admin::add_user)
            .service(handlers::admin::update_user)
            .service(handlers::admin::delete_user)
            .service(handlers::admin::toggle_user_status)
            .service(handlers::admin::get_logs)
            // 日志管理
            .service(handlers::admin::get_log_config)
            .service(handlers::admin::save_log_config)
            .service(handlers::admin::clean_logs)
            // 定时管理
            .service(handlers::admin::get_cron_tasks)
            .service(handlers::admin::create_or_update_cron)
            .service(handlers::admin::toggle_cron)
            .service(handlers::admin::delete_cron)
            // 系统管理
            .service(handlers::admin::get_system_config)
            .service(handlers::admin::change_admin_password)
            .service(handlers::admin::update_admin_username)
            .service(handlers::admin::toggle_guest_register)
            // 用户 API 端点
            .service(handlers::user_api::query_page_data)
            .service(handlers::user_api::reset_baseline)
            .service(handlers::user_api::get_user_config)
            .service(handlers::user_api::save_user_config)
            .service(handlers::user_api::test_notify)
            .service(handlers::user_api::serve_page)
            .service(handlers::user_api::delete_user_by_token)
            // 版本
            .route("/version", web::get().to(get_version))
            // 静态文件
            .route("/", web::get().to(serve_index))
            .route("/{path:.*}", web::get().to(serve_static))
    })
    .bind(format!("{}:{}", config.host, config.port))?
    .run()
    .await
}

async fn get_version() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "version": VERSION
    }))
}

async fn serve_index(state: web::Data<AppState>) -> HttpResponse {
    if let Some(file) = state.static_files.get("index.html") {
        HttpResponse::Ok()
            .content_type(file.content_type)
            .body(file.content)
    } else {
        HttpResponse::NotFound().body("Not Found")
    }
}

async fn serve_static(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    let path = req.match_info().get("path").unwrap_or("");
    
    if let Some(file) = state.static_files.get(path) {
        HttpResponse::Ok()
            .content_type(file.content_type)
            .body(file.content)
    } else {
        if let Some(file) = state.static_files.get("index.html") {
            HttpResponse::Ok()
                .content_type(file.content_type)
                .body(file.content)
        } else {
            HttpResponse::NotFound().body("Not Found")
        }
    }
}
