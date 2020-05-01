use crate::config::Config;
use redis::aio::Connection;
use redis::AsyncCommands;
use redis::RedisResult;

// TODO use connection pool

async fn get_connection() -> RedisResult<Connection> {
    let client = redis::Client::open(Config::new().redis_url)?;
    client.get_async_connection().await
}

pub async fn keys(pattern: Option<&str>) -> RedisResult<Vec<String>> {
    let pattern = pattern.unwrap_or_default();
    let mut connection = get_connection().await?;
    let data = connection.keys(pattern).await?;
    Ok(data)
}

pub async fn set_multiple(tuples: &[(String, String)], expire: Option<usize>) -> RedisResult<()> {
    let mut connection = get_connection().await?;
    let mut pipeline = redis::Pipeline::new();
    tuples.iter().for_each(|(key, value)| {
        pipeline.set(key, value).ignore();
        if let Some(expire) = expire {
            pipeline.expire(key, expire);
        };
    });
    pipeline.atomic().query_async(&mut connection).await?;
    Ok(())
}
pub async fn get_multiple(keys: &[String]) -> RedisResult<Vec<String>> {
    let mut connection = get_connection().await?;
    let mut pipeline = redis::Pipeline::new();
    keys.iter().for_each(|key| {
        pipeline.get(key);
    });
    let data: Vec<String> = pipeline.atomic().query_async(&mut connection).await?;
    Ok(data)
}
