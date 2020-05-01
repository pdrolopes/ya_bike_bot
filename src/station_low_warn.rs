// TODO think of a better name
use crate::bike_service::Station;
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
const INLINE_KEYBOARD_DATA_TTL: usize = 60 * 60 * 6; // 6 horas
const ACTIVE_STATIONS_WARN: &str = "ACTIVE_STATIONS_WARN";
pub const STATION_WARN_TTL: i64 = 60 * 30; // 30 minutes

#[derive(Serialize, Deserialize, Debug)]
pub struct StationWarn {
    uuid: String,
    network_href: String,
    free_bikes: u32,
    id: String,
    pub message_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub chat_id: Option<i64>,
}
impl StationWarn {
    pub fn id(&self) -> String {
        format!("{}:{}", ACTIVE_STATIONS_WARN, self.uuid)
    }

    pub fn should_warn(&self) -> bool {
        let now = Utc::now();
        now.timestamp() - self.updated_at.timestamp() > WARN_INTERVAL_TIME
    }

    pub fn should_delete(&self) -> bool {
        let now = Utc::now();
        now.timestamp() - self.created_at.timestamp() > STATION_WARN_TTL
    }
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
            created_at: Utc::now(),
        }
    }
}

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

pub async fn reply_markups(
    stations: &[Station],
) -> Result<Vec<Option<InlineKeyboardMarkup>>, Exception> {
    let station_warns: Vec<StationWarn> = stations.iter().map(|station| station.into()).collect();
    let reply_markups: Vec<Option<InlineKeyboardMarkup>> = stations
        .iter()
        .zip(station_warns.iter())
        .map(|(station, warn)| reply_markup(station, &warn.uuid))
        .collect();
    let key_value: Vec<(String, String)> = station_warns
        .iter()
        .map(|station_warn| {
            let uuid = &station_warn.uuid;
            let station_warn = serde_json::to_string(&station_warn).unwrap_or_default();
            (uuid.into(), station_warn)
        })
        .collect();
    redis_helper::set_multiple(&key_value, Some(INLINE_KEYBOARD_DATA_TTL)).await?;

    Ok(reply_markups)
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
            "💔 `{}` has lost {} bikes",
            escape(&updated_station.name),
            bold(&free_bikes_diff.abs().to_string())
        ),
        0 => return None,
        1..=i32::MAX => format!(
            "💚 {} has appeard on `{}`!!! It now has {} bikes.",
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
    let (old_station_warns, active_station_warns): (Vec<_>, Vec<_>) =
        redis_helper::get_multiple(&keys)
            .await?
            .into_iter()
            .filter_map(|data| serde_json::from_str(&data).ok())
            .partition(StationWarn::should_delete);
    // Delte old station warns that have passed their ttl
    let old_station_warns_keys: Vec<_> =
        old_station_warns.into_iter().map(|osw| osw.id()).collect();
    redis_helper::del_multiple(&old_station_warns_keys).await?;

    let mut stations_to_be_warned: Vec<StationWarn> = active_station_warns
        .into_iter()
        .filter(StationWarn::should_warn)
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
            station_warn.updated_at = Utc::now();
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
    redis_helper::set_multiple(&saves, None).await?;

    let send_messages: Vec<_> = send_messages
        .iter()
        .map(|send_message| send_message.send())
        .collect();

    log::debug!("{} messages to be sent", &send_messages.len());
    let results: Vec<_> = join_all(send_messages).await;
    results
        .iter()
        .filter_map(|r| r.as_ref().err())
        .for_each(|err| log::error!("Error sending message {:?}", err));
    Ok(())
}
