// TODO think of a better name
use crate::bike_service::Station;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use uuid::Uuid;
const LOW_PERCENTAGE_BIKES: f32 = 0.2; // 20%

impl Station {
    pub fn reply_markup(&self) -> Option<InlineKeyboardMarkup> {
        let free_bikes = self.free_bikes? as f32;
        let empty_slots = self.empty_slots? as f32;
        let show_warn = (free_bikes / (free_bikes + empty_slots)) <= LOW_PERCENTAGE_BIKES;

        if !show_warn {
            return None;
        };
        let uuid = Uuid::new_v4().to_simple().to_string();
        // TODO save information on Redis
        dbg!(&uuid);
        let button = InlineKeyboardButton::callback("Alert!".to_string(), uuid);
        Some(InlineKeyboardMarkup::default().append_row(vec![button]))
    }
}
