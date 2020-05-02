use crate::bike_service;
use bike_service::{Geo, Station};
use teloxide::dispatching::DispatcherHandlerCx;
use teloxide::error_handlers::OnError;
use teloxide::requests::Request;
use teloxide::types::Message;
use teloxide::utils::markdown::{escape, italic, link};
const SMALL_BIKE_AMOUNT: u32 = 6;
const STATION_MAX_TAKE: usize = 5;
const STATION_MIN_TAKE: usize = 3;
const GOOGLE_MAPS_URL: &str = "https://www.google.com/maps";
use crate::station_low_warn::reply_markups;
use std::f64::INFINITY;
use surf::Exception;
use teloxide::types::{Location, ParseMode};
use url::Url;

pub async fn handle(context: &DispatcherHandlerCx<Message>) {
    let DispatcherHandlerCx {
        update: message, ..
    } = &context;

    let location = if let Some(location) = message.location() {
        location
    } else {
        return;
    };

    let stations = match find_near_stations(location).await {
        Ok(stations) => stations,
        Err(err) => {
            log::error!("Error fetching stations {:?}", err);
            context
                .answer("There was a problem to list stations")
                .send()
                .await
                .log_on_error()
                .await;
            return;
        }
    };

    // Calculate take value, to see if we iter 3 or 5 stations
    let is_small_amount: bool = stations
        .iter()
        .take(STATION_MIN_TAKE as usize)
        .map(|s| s.free_bikes.unwrap_or_default()) // defaults to 0
        .sum::<u32>()
        <= SMALL_BIKE_AMOUNT;
    let take = if is_small_amount {
        STATION_MAX_TAKE
    } else {
        STATION_MIN_TAKE
    };
    let stations: Vec<Station> = stations.into_iter().take(take).collect();

    let send_messages_iter = stations.iter().map(|station| {
        context
            .answer(station.message())
            .parse_mode(ParseMode::MarkdownV2)
            .disable_web_page_preview(true)
            .disable_notification(true)
    });
    let reply_markups = reply_markups(&stations).await;
    let send_messages: Vec<_> = if let Ok(reply_markups) = reply_markups {
        send_messages_iter
            .zip(reply_markups.into_iter().filter_map(|rm| rm))
            .map(|(send_message, reply_markup)| send_message.reply_markup(reply_markup))
            .collect()
    } else {
        log::error!("Error creating reply markups");
        send_messages_iter.collect()
    };

    for send_message in send_messages {
        send_message.send().await.log_on_error().await;
    }
}

impl Station {
    fn message(&self) -> String {
        let mut url = Url::parse(GOOGLE_MAPS_URL).unwrap();
        url.query_pairs_mut()
            .append_pair("q", &format!("{},{}", &self.latitude, &self.longitude));
        let name = link(url.as_str(), &escape(&self.name));
        let free_bikes = self
            .free_bikes
            .map_or(String::from("??"), |num| num.to_string());
        let empty_slots = self
            .empty_slots
            .map_or(String::from("??"), |num| num.to_string());
        let description = self
            .extra
            .as_ref()
            .and_then(|extra| extra.description.as_ref().or(extra.address.as_ref()))
            .map(|value| value.clone())
            .unwrap_or_default();
        let description = italic(&escape(&description));
        dbg!(&description);
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
    dbg!(&networks[0]);
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
