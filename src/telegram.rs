const TELEGRAM_BOT_API: &str = "https://api.telegram.org/bot";
use reqwest;
use std::collections::HashMap;

pub struct Telegram {
    token: String,
}

impl Telegram {
    pub fn new(token: String) -> Telegram {
        Telegram { token }
    }
    pub async fn set_webhook(&self, host: String) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}{}/setWebhook?url={}", TELEGRAM_BOT_API, self.token, host);
        let resp = reqwest::get(&url).await?;
        println!("{:?}", resp);
        Ok(())
    }
}
