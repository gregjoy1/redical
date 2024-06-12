use anyhow::{Context, Result};

use redis::Connection;
use std::fs;
use std::thread;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, mpsc};

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

pub fn delete_existing_test_rdb_dump() -> Result<()> {
    let test_rdb_dump_path: PathBuf = [
        std::env::current_dir()?,
        PathBuf::from(format!("test_dump.rdb")),
    ]
    .iter()
    .collect();

    if fs::metadata(&test_rdb_dump_path).is_ok() {
        if let Err(error) = fs::remove_file(&test_rdb_dump_path) {
            panic!("There was a problem removing test_dump.rdb from path: {}, with error: {}", test_rdb_dump_path.display(), error.to_string());
        }
    }

    Ok(())
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

pub fn listen_for_keyspace_events(port: u16, mut handler: impl FnMut(&mut Arc<Mutex<VecDeque<redis::Msg>>>) -> Result<()>) -> Result<()> {
    let (kill_tx, kill_rx): (mpsc::Sender<()>, mpsc::Receiver<()>) = mpsc::channel();

    let mut message_queue = Arc::new(Mutex::new(VecDeque::new()));
    let thread_message_queue = message_queue.clone();

    let mut connection = get_redis_connection(port).unwrap();

    // Enable keyspace pub/sub events:
    // * K - Keyspace events, published with __keyspace@<db>__ prefix.
    // * e - Evicted events (events generated when a key is evicted for maxmemory)
    // * g - Generic commands (non-type specific) like DEL, EXPIRE, RENAME, ...
    // * d - Module key type events
    //
    // Also set in tests/redis_test_config.conf
    redis::cmd("CONFIG")
        .arg("SET")
        .arg(b"notify-keyspace-events")
        .arg("Kegd")
        .execute(&mut connection);

    let join_handle = thread::spawn(move || {
        let mut pub_sub = connection.as_pubsub();

        pub_sub.psubscribe("__key*__:*").unwrap();

        let _ = pub_sub.set_read_timeout(Some(Duration::new(0, 500)));

        loop {
            if let Ok(_) = kill_rx.try_recv() {
                break;
            }

            match pub_sub.get_message() {
                Ok(message) => {
                    thread_message_queue.lock().unwrap().push_back(message);
                },

                Err(error) if error.is_timeout() => {},

                Err(error) => {
                    panic!("Redis pub/sub listener get_message error: #{error}");
                },
            }
        }
    });

    // Give the pub/sub listener thread a moment to get started...
    std::thread::sleep(std::time::Duration::from_millis(50));

    handler(&mut message_queue)?;

    kill_tx.send(()).unwrap();
    join_handle.join().unwrap();

    Ok(())
}
