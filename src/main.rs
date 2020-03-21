use dotenv;
mod bike_service;
use std::env;
use teloxide::prelude::*;
use teloxide::types::{MediaKind, MessageKind};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    run().await;
}

async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting ping_pong_bot!");
    let token = env::var("TOKEN").expect("Missing TOKEN env");

    let bot = Bot::new(token);

    Dispatcher::new(bot)
        .messages_handler(|rx: DispatcherHandlerRx<Message>| {
            rx.for_each(|context| async move {
                let message = &context.update;
                if let MessageKind::Common { media_kind, .. } = &message.kind {
                    if let MediaKind::Location { .. } = media_kind {
                        handle_location_message(context).await
                    } else {
                        context.answer("???").send().await.log_on_error().await
                    }
                }
            })
        })
        .dispatch()
        .await;
}

async fn handle_location_message(context: DispatcherHandlerCx<Message>) {
    if let Ok(networks) = bike_service::fetch_networks().await {
        let network = networks.first();
        log::info!("{:?}", network);
        context.answer("ok").send().await.log_on_error().await;
    } else {
        context
            .answer("achei nada")
            .send()
            .await
            .log_on_error()
            .await;
    }
}
