use dotenv;
mod bike_service;
mod config;
mod error;
mod web_hooks;
use bike_service::{Geo, Station};
use config::Config;
use geoutils;
use std::f64::INFINITY;
use surf::Exception;
use teloxide::prelude::*;
use teloxide::requests::SendChatActionKind;
use teloxide::types::{ButtonRequest, KeyboardButton, Location, ParseMode, ReplyKeyboardMarkup};
use teloxide::utils::markdown::{escape, italic, link};
use tokio;
use url::Url;
const SMALL_BIKE_AMOUNT: u32 = 6;
const STATION_MAX_TAKE: usize = 5;
const STATION_MIN_TAKE: usize = 3;
const GOOGLE_MAPS_URL: &str = "https://www.google.com/maps";

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    run().await;
}

async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting Yet Another Bike Bot");

    let Config {
        telegram_token,
        host,
        port,
        poll,
    } = Config::new();
    let bot = Bot::new(telegram_token);

    let dispatcher =
        Dispatcher::new(bot.clone()).messages_handler(|rx: DispatcherHandlerRx<Message>| {
            rx.for_each(|context| async move {
                let DispatcherHandlerCx { update, bot } = &context;
                let send_action =
                    bot.send_chat_action(update.chat_id(), SendChatActionKind::Typing);
                tokio::spawn(async move { send_action.send().await });
                let message_text = update.text_owned().unwrap_or_default();

                // Handle commands
                if message_text.starts_with("/start") {
                    handle_start(&context).await;
                    return;
                } else if message_text.starts_with("/about") {
                    handle_about(&context).await;
                    return;
                }

                // Log user
                if let Some(user) = update.from() {
                    let mention = user.mention().unwrap_or_default();
                    log::info!("Location sent by: {}, {} ", user.full_name(), mention);
                };
                let answer_messages = if let Some(location) = update.location() {
                    build_near_stations_message(location).await
                } else {
                    return;
                };

                let send_messages = answer_messages.iter().map(|m| {
                    context
                        .answer(m)
                        .parse_mode(ParseMode::MarkdownV2)
                        .disable_web_page_preview(true)
                        .disable_notification(true)
                });
                for send_message in send_messages {
                    send_message.send().await.log_on_error().await;
                }
            })
        });
    if poll {
        dispatcher.dispatch().await;
    } else {
        dispatcher
            .dispatch_with_listener(
                web_hooks::webhook(bot.clone(), &host, port).await,
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
        .parse_mode(ParseMode::MarkdownV2)
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
        .send()
        .await
        .log_on_error()
        .await;
}

async fn build_near_stations_message(location: &Location) -> Vec<String> {
    let stations = match find_near_stations(location).await {
        Ok(stations) => stations,
        Err(err) => {
            log::error!("Error fetching stations {:?}", err);
            return vec![String::from("No stations found.")];
        }
    };
    let is_small_amount: bool = stations
        .iter()
        .take(SMALL_BIKE_AMOUNT as usize)
        .map(|s| s.free_bikes.unwrap_or_default()) // defaults to 0
        .sum::<u32>()
        <= SMALL_BIKE_AMOUNT;
    let take = if is_small_amount {
        STATION_MAX_TAKE
    } else {
        STATION_MIN_TAKE
    };
    let station_messages = stations.iter().take(take).map(|s| s.message()).collect();
    log::debug!("{:?}", station_messages);
    station_messages
}

impl Station {
    fn message(&self) -> String {
        let mut url = Url::parse(GOOGLE_MAPS_URL).unwrap();
        url.query_pairs_mut()
            .append_pair("q", &format!("{},{}", &self.latitude, &self.longitude));
        let name = link(url.as_str(), &self.name);
        let free_bikes = &self
            .free_bikes
            .map_or(String::from("??"), |num| num.to_string());
        let empty_slots = &self
            .empty_slots
            .map_or(String::from("??"), |num| num.to_string());
        let description = self
            .extra
            .as_ref()
            .and_then(|extra| extra.description.clone())
            .unwrap_or_else(String::new);
        let description = italic(&escape(&description));
        format!(
            "`Station   :` {}
`Bikes     :` {}
`Free slot :` {}
{}",
            name, free_bikes, empty_slots, description
        )
    }
}

async fn find_near_stations(location: &Location) -> Result<Vec<Station>, Exception> {
    let user_location = geoutils::Location::new(location.latitude, location.longitude);
    let mut networks = bike_service::fetch_networks().await?;
    networks.sort_by_key(|network| {
        user_location
            .distance_to(&network.location())
            .unwrap_or_else(|_| geoutils::Distance::from_meters(INFINITY))
            .meters() as u32
    });
    let mut stations = if let Some(network) = networks.first() {
        log::debug!("Closest bike network, {}", network.name);
        network.stations().await?
    } else {
        vec![]
    };
    stations.sort_by_key(|station| {
        user_location
            .distance_to(&station.location())
            .unwrap_or_else(|_| geoutils::Distance::from_meters(INFINITY))
            .meters() as u32
    });
    Ok(stations)
}
