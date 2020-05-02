pub mod bike_service;
mod config;
pub mod handle_callback_query;
mod handle_location;
pub mod redis_helper;
pub mod station_low_warn;
mod web_hooks;
use config::Config;
use handle_location::handle as handle_location;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::requests::SendChatActionKind;
use teloxide::types::{
    ButtonRequest, CallbackQuery, KeyboardButton, ParseMode, ReplyKeyboardMarkup,
};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    run().await;
}

async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting Yet Another Bike Bot");

    let config = Config::new();
    let bot = Bot::new(config.telegram_token);
    start_station_warn_loop(bot.clone());

    let dispatcher = Dispatcher::new(bot.clone())
        .messages_handler(|rx: DispatcherHandlerRx<Message>| {
            rx.for_each_concurrent(None, |context| async move {
                let DispatcherHandlerCx { update, bot } = &context;

                //Send action that shows "Typing..."
                let send_action =
                    bot.send_chat_action(update.chat_id(), SendChatActionKind::Typing);
                tokio::spawn(async move { send_action.send().await });

                // Log user
                if let Some(user) = update.from() {
                    let mention = user.mention().unwrap_or_default();
                    log::info!("Message from: {}, {} ", user.full_name(), mention);
                };

                // Handle commands
                let message_text = update.text_owned().unwrap_or_default();
                let message_location = update.location();
                if message_text.starts_with("/start") {
                    handle_start(&context).await;
                    return;
                } else if message_text.starts_with("/about") {
                    handle_about(&context).await;
                    return;
                } else if message_location.is_some() {
                    handle_location(&context).await;
                } else {
                    handle_start(&context).await;
                }
            })
        })
        .callback_queries_handler(|rx: DispatcherHandlerRx<CallbackQuery>| {
            rx.for_each_concurrent(None, |context| async move {
                let user = &context.update.from;
                let mention = user.mention().unwrap_or_default();
                log::info!("Callback query from: {}, {} ", user.full_name(), mention);

                handle_callback_query::handle(&context).await;
            })
        });
    if config.poll {
        dispatcher.dispatch().await;
    } else {
        dispatcher
            .dispatch_with_listener(
                web_hooks::webhook(bot.clone(), &config.host, config.port).await,
                LoggingErrorHandler::with_custom_text("An error from the update listener"),
            )
            .await
    };
}

async fn handle_start(context: &DispatcherHandlerCx<Message>) {
    let location_button = KeyboardButton::new("Send location").request(ButtonRequest::Location);
    let keyboard = ReplyKeyboardMarkup::default()
        .resize_keyboard(true)
        .append_row(vec![location_button]);
    context
        .answer("Send me a Location so I can send you information from near bike stations")
        .reply_markup(keyboard)
        .send()
        .await
        .log_on_error()
        .await;
}

async fn handle_about(context: &DispatcherHandlerCx<Message>) {
    let message = "
Created by [Pedro Lopes](https://t.me/pdrolopes)
Code is available on [Github](https://github.com/pdrolopes/ya_bike_bot)

Information from the bike stations are fetched from [CityBikes](https://citybik.es/)\\.
This Bot was made with [Teloxide](https://github.com/teloxide/teloxide) library
    ";
    context
        .answer(message)
        .parse_mode(ParseMode::MarkdownV2)
        .disable_web_page_preview(true)
        .send()
        .await
        .log_on_error()
        .await;
}

// TODO name this better
fn start_station_warn_loop(bot: Arc<Bot>) {
    log::info!("Started loop");
    tokio::spawn(async move {
        loop {
            let bot = bot.clone();
            // TODO Moved redis to a centrlized place.
            station_low_warn::check_active_warn_stations(bot)
                .await
                .unwrap_or_else(|err| {
                    log::error!("While checking active station warns. {:?}", err)
                });
            tokio::time::delay_for(Duration::new(60, 0)).await
        }
    });
}
