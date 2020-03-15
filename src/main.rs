use actix_web::{get, middleware, web, App, HttpServer, Responder};
mod config;
mod telegram;
use config::Config;
use dotenv;
use telegram::Telegram;

#[get("/{id}/{name}/index.html")]
async fn index(info: web::Path<(u32, String)>) -> impl Responder {
    format!("Hello {}! id:{}", info.1, info.0)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let Config { token, host } = Config::new().expect("Missing env variables");
    let telegram = Telegram::new(token);
    telegram
        .set_webhook(host)
        .await
        .expect("Error while setting webhook");
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(index)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
