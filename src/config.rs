use std::env;
pub struct Config {
    pub telegram_token: String,
    pub poll: bool,
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn new() -> Config {
        let token = env::var("TOKEN").expect("Missing TOKEN env");
        let host = env::var("HOST").expect("Missing HOST env");
        let poll: bool = env::var("POLL")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .expect("non boolean value");
        let port: u16 = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .expect("non interger value");
        Config {
            telegram_token: token,
            poll,
            host,
            port,
        }
    }
}
