use actix_cors::Cors;
use actix_web::{web, App, HttpServer, middleware, HttpRequest, HttpResponse};

use tracing_subscriber::EnvFilter;
use std::sync::Arc;
use std::io::{self, Write, IsTerminal};

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
    pub static_files: Arc<std::collections::HashMap<String, StaticFile>>,
}

// ═══════════════════════════════════════════════
// 工具函数
// ═══════════════════════════════════════════════

fn read_line() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn get_pid() -> Option<u32> {
    let pid_file = format!("{}/unicom.pid", get_install_dir());
    if let Ok(content) = std::fs::read_to_string(&pid_file) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            #[cfg(target_os = "windows")]
            {
                // Windows: check if process exists via tasklist
                let output = std::process::Command::new("tasklist")
                    .args(["/FI", &format!("PID eq {}", pid), "/NH"])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .output();
                if let Ok(out) = output {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    // tasklist returns "INFO: no tasks running" if not found
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
    let config = config::Config::load().expect("Failed to load config");
    println!("Unicom v{}", VERSION);
    println!("  地址: {}:{}", config.host, config.port);
    println!("  数据库: {}", config.database_path);
    match get_pid() {
        Some(pid) => {
            println!("  状态: ✅ 运行中 (PID: {})", pid);
            
            #[cfg(not(target_os = "windows"))]
            {
            // 读取 /proc/[pid]/status 获取资源信息
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
                        println!("  CPU: {:.1}%", cpu_pct);
                    } else {
                        println!("  CPU: 0.0%");
                    }
                }
            }
            
            // 运行时间
            if let Ok(content) = std::fs::read_to_string(&stat_path) {
                let fields: Vec<&str> = content.split_whitespace().collect();
                if fields.len() > 21 {
                    let starttime: u64 = fields[21].parse().unwrap_or(0);
                    let uptime_path = "/proc/uptime";
                    if let Ok(uptime_content) = std::fs::read_to_string(uptime_path) {
                        let uptime_secs: f64 = uptime_content
                            .split_whitespace()
                            .next()
                            .unwrap_or("0")
                            .parse()
                            .unwrap_or(0.0);
                        let hz = 100.0; // 假设 100 Hz
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
        println!("服务已在运行中 (PID: {})", get_pid().unwrap());
        return;
    }
    // Android: 使用 service.d 脚本
    let android_script = "/data/adb/service.d/unicom.sh";
    if std::path::Path::new(android_script).exists() {
        println!("启动服务...");
        std::process::Command::new("sh")
            .args([android_script, "start"])
            .status()
            .ok();
        std::thread::sleep(std::time::Duration::from_secs(3));
        if is_running() {
            println!("✅ 启动成功 (PID: {})", get_pid().unwrap());
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
        // Windows: use CREATE_NO_WINDOW to detach from console
        use std::os::windows::process::CommandExt;
        let result = std::process::Command::new(&exe)
            .current_dir(&install_dir)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn();
        match result {
            Ok(child) => {
                // Write PID manually since we can't rely on kill -0 check
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
        std::process::Command::new(&exe)
            .current_dir(&install_dir)
            .spawn()
            .expect("Failed to start");
        std::thread::sleep(std::time::Duration::from_secs(1));
        if is_running() {
            println!("✅ 启动成功 (PID: {})", get_pid().unwrap());
        } else {
            println!("❌ 启动失败，请查看日志");
        }
    }
}

fn cmd_stop() {
    // Android: 优先使用 service.d 脚本
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
                // Windows: use taskkill
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
    let config = config::Config::load().expect("Failed to load config");
    let conn = rusqlite::Connection::open(&config.database_path).expect("Failed to open database");
    conn.execute_batch("PRAGMA foreign_keys = ON;").ok();

    // 生成随机用户名和密码
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let chars: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    // 用种子混合索引，确保每个字符都不同
    let mix = |val: u64| -> char {
        let h = val.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        chars[(h >> 33) as usize % chars.len()] as char
    };
    let username: String = (0..6).map(|i| mix(seed.wrapping_add(i as u64 * 7 + 1))).collect();
    let password: String = (0..16).map(|i| mix(seed.wrapping_add(i as u64 * 13 + 31))).collect();
    let username = format!("admin_{}", username);

    let hashed = bcrypt::hash(&password, bcrypt::DEFAULT_COST).unwrap();

    // 检查是否有管理员
    let admin_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM admins", [], |row| row.get(0))
        .unwrap_or(0);

    let result = if admin_count > 0 {
        // 更新已有管理员
        conn.execute(
            "UPDATE admins SET username = ?1, password = ?2 WHERE id = (SELECT id FROM admins LIMIT 1)",
            rusqlite::params![username, hashed],
        )
    } else {
        // 创建新管理员
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
        format!("运行中 (PID: {})", get_pid().unwrap())
    } else {
        "未运行".to_string()
    };

    println!("\n┌───────────────────────────────────┐");
    println!("│     Unicom 联通流量查询系统       │");
    println!("│            v{:<20}│", VERSION);
    println!("├───────────────────────────────────┤");
    println!("│  状态: {:<26}│", status);
    println!("├───────────────────────────────────┤");
    println!("│  1. 启动服务                      │");
    println!("│  2. 重启服务                      │");
    println!("│  3. 停止服务                      │");
    println!("│  4. 运行状态                      │");
    println!("│  5. 管理账密                      │");
    println!("│  6. 检查更新                      │");
    println!("│  7. 卸载程序                      │");
    println!("│  0. 退出                          │");
    println!("└───────────────────────────────────┘");
    print!("请选择: ");
    io::stdout().flush().unwrap();
}

fn show_admin_menu() {
    println!("\n┌─────────────────────────────────────┐");
    println!("│           管理员账密                │");
    println!("├─────────────────────────────────────┤");
    println!("│  1. 重置管理员账密                  │");
    println!("│  2. 查看管理员列表                  │");
    println!("│  0. 返回                            │");
    println!("└─────────────────────────────────────┘");
    print!("请选择: ");
    io::stdout().flush().unwrap();
}

fn list_admins() {
    let config = config::Config::load().expect("Failed to load config");
    let conn = rusqlite::Connection::open(&config.database_path).expect("Failed to open database");

    println!("\n=== 管理员列表 ===");
    let mut stmt = conn
        .prepare("SELECT id, username, real_name FROM admins")
        .expect("Failed to prepare");

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    }).expect("Failed to query");

    let mut count = 0;
    for row in rows {
        let (id, username, real_name) = row.unwrap();
        println!("  {}. {} ({})", id, username, if real_name.is_empty() { "-" } else { &real_name });
        count += 1;
    }

    if count == 0 {
        println!("  暂无管理员");
    }
}

fn do_update() {
    let install_dir = get_install_dir();
    println!("\n=== 更新 Unicom ===");
    println!("安装目录: {}", install_dir);
    println!();
    println!("请执行以下命令更新:");
    println!("  curl -fsSL https://ghfast.top/https://raw.githubusercontent.com/amuae/unicom/main/deploy.sh | sh");
    println!();
    print!("是否现在执行？(y/N): ");
    io::stdout().flush().unwrap();

    if read_line().to_lowercase() == "y" {
        println!("正在更新...");
        std::process::Command::new("sh")
            .args(["-c", "curl -fsSL https://ghfast.top/https://raw.githubusercontent.com/amuae/unicom/main/deploy.sh | sh"])
            .status()
            .ok();
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
    io::stdout().flush().unwrap();

    if read_line() == "yes" {
        // 停止服务
        cmd_stop();

        // 删除文件
        println!("删除文件...");
        std::fs::remove_dir_all(&install_dir).ok();

        // 删除服务脚本
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
            _ => println!("无效选择"),
        }
    }
}

fn interactive_menu() {
    loop {
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
            _ => println!("无效选择"),
        }
    }
}

// ═══════════════════════════════════════════════
// 主入口
// ═══════════════════════════════════════════════

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 切换工作目录到二进制所在目录，确保相对路径（config.toml、unicom.db、unicom.pid）正确
    let install_dir = get_install_dir();
    std::env::set_current_dir(&install_dir).ok();

    let args: Vec<String> = std::env::args().collect();

    // 有参数：快捷指令模式
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

    // 无参数：检测是否有终端
    // 有终端 → 交互菜单，无终端（systemctl） → 直接启动服务
    let has_tty = io::stdin().is_terminal();
    
    if has_tty {
        // 有终端：交互菜单模式
        interactive_menu();
    }

    // 直接启动服务（systemctl 或从菜单中选择启动）

    // ═══════════════════════════════════════════════
    // 以下为正常服务启动（从 menu 1 或直接 ./unicom 会到这里）
    // ═══════════════════════════════════════════════

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
    tracing::info!("Database pool initialized (max_size=10, WAL mode)");

    // 加载静态文件
    let static_files = Arc::new(static_files::load_static_files());
    tracing::info!("Loaded {} static files", static_files.len());

    // 创建应用状态
    let app_state = AppState {
        db: db.clone(),
        config: config.clone(),
        static_files: static_files.clone(),
    };

    // 启动定时任务
    let cron_state = app_state.clone();
    tokio::spawn(async move {
        cron::start_cron_jobs(cron_state).await;
    });

    // 写入 PID 文件
    let pid = std::process::id();
    std::fs::write("unicom.pid", pid.to_string()).ok();

    tracing::info!("Starting server at {}:{}", config.host, config.port);
    println!("Unicom v{} 启动成功 http://{}:{}", VERSION, config.host, config.port);

    // 启动 HTTP 服务器
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

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
        // SPA fallback
        if let Some(file) = state.static_files.get("index.html") {
            HttpResponse::Ok()
                .content_type(file.content_type)
                .body(file.content)
        } else {
            HttpResponse::NotFound().body("Not Found")
        }
    }
}
