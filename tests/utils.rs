use anyhow::{Context, Result};

use redis::Connection;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

/// Ensure child process is killed both on normal exit and when panicking due to a failed test.
pub struct ChildGuard {
    name: &'static str,
    child: std::process::Child,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Err(e) = self.child.kill() {
            println!("Could not kill {}: {e}", self.name);
        }
        if let Err(e) = self.child.wait() {
            println!("Could not wait for {}: {e}", self.name);
        }
    }
}

pub fn start_redis_server_with_module(module_name: &str, port: u16) -> Result<ChildGuard> {
    let extension = if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };

    let profile = if cfg!(not(debug_assertions)) {
        "release"
    } else {
        "debug"
    };

    let module_path: PathBuf = [
        std::env::current_dir()?,
        PathBuf::from(format!("target/{profile}/lib{module_name}.{extension}")),
    ]
    .iter()
    .collect();

    let test_config_path: PathBuf = [
        std::env::current_dir()?,
        PathBuf::from(format!("tests/redis_test_config.conf")),
    ]
    .iter()
    .collect();

    assert!(fs::metadata(&test_config_path)
        .with_context(|| format!("Loading redis test config: {}", test_config_path.display()))?
        .is_file());

    assert!(fs::metadata(&module_path)
        .with_context(|| format!("Loading redis module: {} (prepend `cargo build &&` to `cargo test` to ensure artifacts exist)", module_path.display()))?
        .is_file());

    let test_config_path = format!("{}", test_config_path.display());
    let module_path = format!("{}", module_path.display());

    let args = &[
        test_config_path.as_str(),
        "--port",
        &port.to_string(),
        "--loadmodule",
        module_path.as_str(),
    ];

    let redis_server = Command::new("redis-server")
        .args(args)
        .spawn()
        .map(|child| ChildGuard {
            name: "redis-server",
            child,
        })?;

    Ok(redis_server)
}

// Get connection to Redis
pub fn get_redis_connection(port: u16) -> Result<Connection> {
    let client = redis::Client::open(format!("redis://127.0.0.1:{port}/"))?;

    loop {
        let res = client.get_connection();
        match res {
            Ok(con) => return Ok(con),
            Err(e) => {
                if e.is_connection_refusal() {
                    // Redis not ready yet, sleep and retry
                    std::thread::sleep(Duration::from_millis(50));
                } else {
                    return Err(e.into());
                }
            }
        }
    }
}
