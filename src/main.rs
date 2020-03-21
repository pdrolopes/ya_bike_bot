use dotenv;
mod bike_service;
use bike_service::{Geo, Station};
use geoutils;
use std::env;
use std::f64::INFINITY;
use teloxide::prelude::*;
use teloxide::types::{Location, MediaKind, MessageKind};

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
    match find_near_stations(location).await {
        Ok(stations) => {
            let first_five: Vec<String> = stations.iter().map(|s| s.name.clone()).take(5).collect();
            log::debug!("{:?}", first_five);
            format!("Found {} stations. {:?}", stations.len(), first_five)
        }
        Err(err) => {
            log::error!("Error fetching stations {:?}", err);
            "No stations found".to_string()
        }
    }
}

type AsyncError = Box<dyn std::error::Error + Send + Sync + 'static>;

async fn find_near_stations(location: &Location) -> Result<Vec<Station>, AsyncError> {
    let user_location = geoutils::Location::new(location.latitude, location.longitude);
    log::info!("pedro");
    let mut networks = bike_service::fetch_networks().await?;
    log::info!("{:?}", networks.len());
    networks.sort_by_key(|network| {
        user_location
            .distance_to(&network.location())
            .unwrap_or(geoutils::Distance::from_meters(INFINITY))
            .meters() as u32
    });
    //TODO remove unwrap
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
            .unwrap_or(geoutils::Distance::from_meters(INFINITY))
            .meters() as u32
    });
    Ok(stations)
}
