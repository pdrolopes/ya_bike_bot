use crate::bike_service::Station;
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct StationWarn {
    pub uuid: String,
    pub message_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub chat_id: i64,
    pub station_info: StationReminderInfo,
}

#[derive(Serialize, Deserialize, Debug, From)]
pub enum CallbackData {
    StartStationReminder(StationReminderInfo),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StationReminderInfo {
    pub uuid: String,
    pub network_href: String,
    pub free_bikes: u32,
    pub id: String,
}

// impl TryFrom<Station> for StationReminderInfo {
//     fn try_from(station: Station) -> Result<Self> {
//         let network_href = station.network_href?;
//         let free_bikes = station.free_bikes?;
//         let id = station.id;
//         let uuid = Uuid::new_v4().to_simple().to_string();
//         StationReminderInfo {
//             uuid,
//             network_href,
//             free_bikes,
//             id,
//         }
//     }
// }
