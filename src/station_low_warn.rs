// TODO think of a better name
use crate::bike_service::Station;
use crate::handle_callback_query::ACTIVE_STATIONS_WARN;
use crate::redis_helper;
use chrono::prelude::*;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::convert::From;
use std::sync::Arc;
use surf::Exception;
use teloxide::prelude::*;
use teloxide::requests::SendMessage;
use teloxide::types::ParseMode;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::utils::markdown::{bold, escape};
use uuid::Uuid;
const LOW_PERCENTAGE_BIKES: f32 = 0.8; // 20%
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
    uuid: String,
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
        let uuid = Uuid::new_v4().to_simple().to_string();
        StationWarn {
            uuid,
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
    let station_warns: Vec<StationWarn> = stations.iter().map(|station| station.into()).collect();
    let reply_markups: Vec<Option<InlineKeyboardMarkup>> = stations
        .iter()
        .zip(station_warns.iter())
        .map(|(station, warn)| reply_markup(station, &warn.uuid))
        .collect();
    let mut new_pipeline = redis::Pipeline::new();
    let pipeline_set: &mut redis::Pipeline =
        station_warns
            .iter()
            .fold(&mut new_pipeline, |pipe, station_warn| {
                let uuid = &station_warn.uuid;
                let station_warn = serde_json::to_string(&station_warn).unwrap();
                pipe.set(uuid, station_warn)
                    .ignore()
                    .expire(uuid, 60 * 60 * 12) // half day
            });

    let client = redis::Client::open(crate::config::Config::new().redis_url).unwrap(); // TODO set redis addres to env variable
    let mut con = client.get_async_connection().await.unwrap();
    pipeline_set.query_async::<_, ()>(&mut con).await.unwrap();
    reply_markups
}

pub fn build_telegram_message(
    station_warn: &StationWarn,
    updated_station: &Station,
    bot: Arc<Bot>,
) -> Option<SendMessage> {
    let updated_station_free_bikes = updated_station.free_bikes?;
    let free_bikes_diff = updated_station_free_bikes as i32 - station_warn.free_bikes as i32;
    let message = match free_bikes_diff {
        i32::MIN..=-1 => format!(
            "💔 `Station: {}` has lost {} bikes",
            escape(&updated_station.name),
            bold(&free_bikes_diff.abs().to_string())
        ),
        0 => return None,
        1..=i32::MAX => format!(
            "💚 {} has appeard on `Station: {}`!!! It now has {} bikes.",
            bold(&free_bikes_diff.to_string()),
            escape(&updated_station.name),
            bold(&updated_station_free_bikes.to_string())
        ),
    };

    // Build telegram message
    let chat_id = station_warn.chat_id.unwrap_or_default();
    let message_id = station_warn.message_id.unwrap_or_default();

    let send_message = bot
        .send_message(chat_id, message)
        .reply_to_message_id(message_id)
        .parse_mode(ParseMode::MarkdownV2);
    Some(send_message)
}
pub async fn check_active_warn_stations(bot: Arc<Bot>) -> Result<(), Exception> {
    let keys = redis_helper::keys(Some(&format!("{}*", ACTIVE_STATIONS_WARN))).await?;
    log::info!("Found {} station messages to be warned", &keys.len());
    let data = redis_helper::get_multiple(&keys).await?;
    let now = Utc::now();
    let mut stations_to_be_warned: Vec<StationWarn> = data
        .into_iter()
        .filter_map(|data| serde_json::from_str(&data).ok())
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
    let send_messages: Vec<_> = stations_to_be_warned
        .iter_mut()
        .zip(updated_stations.iter())
        .filter_map(|(station_warn, updated_station)| {
            if let Ok(updated_station) = updated_station {
                Some((station_warn, updated_station))
            } else {
                None
            }
        })
        .map(|(station_warn, updated_station)| {
            let send_message = build_telegram_message(station_warn, updated_station, bot.clone());
            // updated station warn info
            station_warn.updated_at = now;
            station_warn.free_bikes = updated_station.free_bikes.unwrap_or_default();

            send_message
        })
        .filter_map(|message| message)
        .collect();

    let saves: Vec<(String, String)> = stations_to_be_warned
        .iter()
        .map(|station_warn| {
            let key = format!("{}:{}", ACTIVE_STATIONS_WARN, station_warn.uuid);
            let data = serde_json::to_string(station_warn).unwrap_or_default();
            (key, data)
        })
        .collect();
    redis_helper::set_multiple(&saves).await?;

    let send_messages: Vec<_> = send_messages
        .iter()
        .map(|send_message| send_message.send())
        .collect();

    log::debug!("{} messages to be sent", &send_messages.len());
    join_all(send_messages).await;
    Ok(())
}
