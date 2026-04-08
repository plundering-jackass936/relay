use std::process::Command;

fn main() {
    // Git commit hash
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Build date
    let build_date = chrono::Local::now().format("%Y-%m-%d").to_string();

    // Rust version
    let rust_version = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Target triple
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=RELAY_GIT_HASH={git_hash}");
    println!("cargo:rustc-env=RELAY_BUILD_DATE={build_date}");
    println!("cargo:rustc-env=RELAY_RUST_VERSION={rust_version}");
    println!("cargo:rustc-env=RELAY_TARGET={target}");
}
