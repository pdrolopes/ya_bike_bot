// TODO think of a better name
use crate::bike_service::Station;
use crate::config::Config;
use chrono::prelude::*;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::convert::From;
use std::sync::Arc;
use surf::Exception;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use uuid::Uuid;
const LOW_PERCENTAGE_BIKES: f32 = 0.2; // 20%
const WARN_INTERVAL_TIME: i64 = (60 * 5) - 5; // ~= 5 minutes

fn reply_markup(station: &Station, uuid: &str) -> Option<InlineKeyboardMarkup> {
    let free_bikes = station.free_bikes? as f32;
    let empty_slots = station.empty_slots? as f32;
    let show_warn = (free_bikes / (free_bikes + empty_slots)) <= LOW_PERCENTAGE_BIKES;

    if !show_warn {
        return None;
    };

    let button = InlineKeyboardButton::callback("Alert!".to_string(), uuid.into());
    Some(InlineKeyboardMarkup::default().append_row(vec![button]))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StationWarn {
    network_href: String,
    free_bikes: u32,
    id: String,
    pub message_id: Option<i32>,
    pub updated_at: DateTime<Utc>,
    pub chat_id: Option<i64>,
}

impl From<&Station> for StationWarn {
    fn from(station: &Station) -> Self {
        let network_href = station.network_href.as_ref().unwrap().into(); // TODO remove unwrap
        let free_bikes = station.free_bikes.unwrap();
        let id = station.id.clone();
        StationWarn {
            network_href,
            free_bikes,
            id,
            message_id: None,
            chat_id: None,
            updated_at: Utc::now(),
        }
    }
}

pub async fn reply_markups(stations: &[Station]) -> Vec<Option<InlineKeyboardMarkup>> {
    let uuids: Vec<String> = stations
        .iter()
        .map(|_| Uuid::new_v4().to_simple().to_string())
        .collect();
    let reply_markups: Vec<Option<InlineKeyboardMarkup>> = stations
        .iter()
        .zip(uuids.iter())
        .map(|(station, uuid)| reply_markup(station, &uuid))
        .collect();
    let mut new_pipeline = redis::Pipeline::new();
    let pipeline_set: &mut redis::Pipeline = stations
        .iter()
        .zip(uuids.into_iter())
        .map(|(station, uuid)| {
            dbg!(&uuid);
            let station_warn: StationWarn = station.into();
            let station_warn = serde_json::to_string(&station_warn).unwrap();
            (uuid, station_warn)
        })
        .fold(&mut new_pipeline, |pipe, (uuid, station)| {
            pipe.set(&uuid, station).ignore().expire(uuid, 60 * 60 * 12) // half day
        });

    let client = redis::Client::open(crate::config::Config::new().redis_url).unwrap(); // TODO set redis addres to env variable
    let mut con = client.get_async_connection().await.unwrap();
    pipeline_set.query_async::<_, ()>(&mut con).await.unwrap();
    reply_markups
}

pub async fn check_active_warn_stations(bot: Arc<Bot>) {
    let client = redis::Client::open(Config::new().redis_url).unwrap(); // TODO set redis addres to env variable
    let mut con = client.get_async_connection().await.unwrap();
    let keys: Vec<String> = redis::AsyncCommands::keys(&mut con, "ACTIVE*")
        .await
        .unwrap();
    log::info!("Found {} station messages to be warned", &keys.len());
    let pipeline = redis::Pipeline::new();
    let data: Vec<String> = keys
        .iter()
        .fold(pipeline, |mut pipe, key| pipe.get(key).to_owned())
        .atomic()
        .query_async(&mut con)
        .await
        .unwrap();
    let now = Utc::now();
    let stations_to_be_warned: Vec<StationWarn> = data
        .into_iter()
        .map(|d| serde_json::from_str(&d).unwrap())
        .filter(|d: &StationWarn| now.timestamp() - d.updated_at.timestamp() > WARN_INTERVAL_TIME)
        .collect();
    log::info!(
        "{} StationWarn are older than 5 minutes",
        &stations_to_be_warned.len()
    ); // TODO remove this
    let updated_stations: Vec<_> = stations_to_be_warned
        .iter()
        .map(|w| Station::fetch(&w.id, &w.network_href))
        .collect();
    let updated_stations: Vec<Result<Station, Exception>> = join_all(updated_stations).await;

    // TODO CHECK any diff on free bikes
    // TODO update station warn free_bikes prop and save on redis
    // TODO Send message to user if it has changed

    // log::debug!("{:?} stations updated", update_stations);
    // for d in stations_to_be_warned.iter() {
    //     let chat_id = d.chat_id.unwrap();
    //     let message_id = d.message_id.unwrap();
    //     bot.send_message(chat_id, "Test message")
    //         .reply_to_message_id(message_id)
    //         .send()
    //         .await
    //         .log_on_error()
    //         .await;
    // }
}
