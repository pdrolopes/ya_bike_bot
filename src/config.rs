use std::env;
pub struct Config {
    pub token: String,
    pub host: String,
}
impl Config {
    pub fn new() -> Result<Config, Box<dyn std::error::Error>> {
        let token = env::var("TOKEN")?;
        let host = env::var("HOST")?;
        Ok(Config { token, host })
    }
}
