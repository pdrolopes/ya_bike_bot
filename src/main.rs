use dotenv;
mod bike_service;
use bike_service::{Geo, Station};
use geoutils;
use std::env;
use std::f64::INFINITY;
use surf::Exception;
use teloxide::prelude::*;
use teloxide::types::{Location, MediaKind, MessageKind};
const SMALL_BIKE_AMOUNT: u32 = 6;
const STATION_MAX_TAKE: usize = 5;
const STATION_MIN_TAKE: usize = 3;

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
                let answer_message = if let MessageKind::Common { media_kind, .. } = &message.kind {
                    if let MediaKind::Location { location } = media_kind {
                        build_near_stations_message(location).await
                    } else {
                        "???".to_string()
                    }
                } else {
                    "???".to_string()
                };
                context
                    .answer(answer_message)
                    .send()
                    .await
                    .log_on_error()
                    .await
            })
        })
        .dispatch()
        .await;
}
async fn build_near_stations_message(location: &Location) -> String {
    let stations = match find_near_stations(location).await {
        Ok(stations) => stations,
        Err(err) => {
            log::error!("Error fetching stations {:?}", err);
            return "No stations found.".to_string();
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
    let station_names: Vec<String> = stations.iter().map(|s| s.name.clone()).take(take).collect();
    log::debug!("{:?}", station_names);
    format!("Found {} stations. {:?} ", stations.len(), station_names,)
}

async fn find_near_stations(location: &Location) -> Result<Vec<Station>, Exception> {
    let user_location = geoutils::Location::new(location.latitude, location.longitude);
    let mut networks = bike_service::fetch_networks().await?;
    log::info!("{:?}", networks.len());
    networks.sort_by_key(|network| {
        user_location
            .distance_to(&network.location())
            .unwrap_or_else(|_| geoutils::Distance::from_meters(INFINITY))
            .meters() as u32
    });
    // log::info!("{:?}", network);
    let mut stations = if let Some(network) = networks.first() {
        network.stations().await?
    } else {
        vec![]
    };
    log::info!("{:?}", stations.len());
    stations.sort_by_key(|station| {
        user_location
            .distance_to(&station.location())
            .unwrap_or_else(|_| geoutils::Distance::from_meters(INFINITY))
            .meters() as u32
    });
    Ok(stations)
}
