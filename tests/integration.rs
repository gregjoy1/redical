use crate::utils::{get_redis_connection, start_redis_server_with_module};
use anyhow::Context;
use anyhow::Result;
use redis::Value;
use redis::{RedisError, RedisResult};

mod utils;

// Run with:
//  cargo test -- --include-ignored
//  cargo test --ignored

#[test]
#[ignore]
fn test_set() -> Result<()> {
    let port: u16 = 6479;
    let _guards = vec![start_redis_server_with_module("redical", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

    redis::cmd("set").arg(&["key", "value"]).query(&mut con)?;

    let res: Vec<String> = redis::cmd("get")
        .arg(&["key"])
        .query(&mut con)
        .with_context(|| "failed to run get")?;

    assert_eq!(res, vec!["value"]);

    let res: Result<Vec<i32>, RedisError> =
        redis::cmd("set").arg(&["key"]).query(&mut con);
    if res.is_ok() {
        return Err(anyhow::Error::msg("Should return an error"));
    }

    Ok(())
}
