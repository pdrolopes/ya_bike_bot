use super::station_low_warn::StationWarn;
use chrono::prelude::*;
use redis::AsyncCommands;
use teloxide::dispatching::DispatcherHandlerCx;
use teloxide::error_handlers::OnError;
use teloxide::requests::Request;
use teloxide::types::{CallbackQuery, ChatId, ChatOrInlineMessage};
pub const ACTIVE_STATIONS_WARN: &str = "ACTIVE_STATIONS_WARN";

pub async fn handle(context: &DispatcherHandlerCx<CallbackQuery>) {
    let DispatcherHandlerCx { update, bot } = &context;
    let uuid_data = match &update.data {
        Some(uuid) => uuid,
        None => return,
    };
    let message = match &update.message {
        Some(m) => m,
        None => return,
    };
    let client = redis::Client::open(crate::config::Config::new().redis_url).unwrap(); // TODO set redis addres to env variable
    let mut con = client.get_async_connection().await.unwrap();
    let data: String = redis::AsyncCommands::get(&mut con, uuid_data)
        .await
        .unwrap();
    let mut data: StationWarn = serde_json::from_str(&data).unwrap();
    data.message_id = Some(message.id);
    data.updated_at = Utc::now();
    data.chat_id = Some(message.chat.id);
    let data = serde_json::to_string(&data).unwrap();
    con.set_ex::<_, _, ()>(
        format!("{}:{}", ACTIVE_STATIONS_WARN, uuid_data),
        data,
        30 * 60,
    )
    .await
    .unwrap();
    bot.edit_message_reply_markup(ChatOrInlineMessage::Chat {
        chat_id: ChatId::Id(message.chat.id),
        message_id: message.id,
    })
    .send()
    .await
    .log_on_error()
    .await;
    bot.answer_callback_query(&update.id)
        .text("I will warn you if this station has any changes in the next 30 minutes")
        .send()
        .await
        .unwrap();
}
