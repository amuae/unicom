fn main() {
    // 生成编译版本号（Unix 时间戳）
    let version = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    println!("cargo:rustc-env=BUILD_VERSION={}", version);

    // 前端文件变化时重新编译
    println!("cargo:rerun-if-changed=../dist/index.html");
    println!("cargo:rerun-if-changed=src/build.rs");
    println!("cargo:rerun-if-changed=src/static_files.rs");
}
