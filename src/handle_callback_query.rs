use super::models::StationWarn;
use crate::models::CallbackData;
use crate::redis_helper;
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use std::sync::Arc;
use teloxide::dispatching::DispatcherHandlerCx;
use teloxide::prelude::*;
use teloxide::requests::Request;
use teloxide::types::{CallbackQuery, ChatId, ChatOrInlineMessage};
use uuid::Uuid;

pub async fn handle(context: &DispatcherHandlerCx<CallbackQuery>) {
    let DispatcherHandlerCx { update, bot } = &context;
    let result = create_station_warn(update, bot.clone()).await;
    let message = match result {
        Ok(_) => "I will warn you if this station has any changes in the next 30 minutes",
        Err(err) => {
            log::error!("Problem handling callback query. Err: `{:?}`", err);
            "There was a problem. :("
        }
    };

    bot.answer_callback_query(&update.id)
        .text(message)
        .send()
        .await
        .log_on_error()
        .await;
}

async fn create_station_warn(callback_query: &CallbackQuery, bot: Arc<Bot>) -> Result<()> {
    let callback_data = callback_query
        .data
        .as_ref()
        .ok_or(anyhow!("Missing uuid on callback data"))?;
    let message = callback_query
        .message
        .as_ref()
        .ok_or(anyhow!("Missing message information on callback data"))?;

    let data: String = redis_helper::get(&callback_data).await?;
    let data: CallbackData = serde_json::from_str(&data)?;
    let station_info = match data {
        CallbackData::StartStationReminder(value) => value,
        _ => return Err(anyhow!("Callback data can't be parsed")),
    };

    let station_warn = StationWarn {
        station_info,
        uuid: Uuid::new_v4().to_simple().to_string(),
        message_id: message.id,
        updated_at: Utc::now(),
        created_at: Utc::now(),
        chat_id: message.chat.id,
    };

    let key = station_warn.id();
    let data = serde_json::to_string(&station_warn)?;
    redis_helper::set_multiple(&vec![(key, data)], None).await?;

    bot.edit_message_reply_markup(ChatOrInlineMessage::Chat {
        chat_id: ChatId::Id(message.chat.id),
        message_id: message.id,
    })
    .send()
    .await
    .log_on_error()
    .await;
    Ok(())
}
