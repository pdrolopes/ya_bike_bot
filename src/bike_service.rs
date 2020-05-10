use serde::{Deserialize, Serialize};
use surf::Exception;
const CITYBIKES_HOST: &str = "http://api.citybik.es";
const NETWORKS_HREF: &str = "/v2/networks";
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BikeServiceError {
    #[error("Network with name:`{0}` does not have href value")]
    InvalidBikeNetwork(String),
    #[error("Station with id:`{0}` not found")]
    StationNotFound(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Location {
    latitude: f64,
    longitude: f64,
    city: String,
    country: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Network {
    href: Option<String>,
    location: Location,
    pub name: String,
    stations: Option<Vec<Station>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Station {
    pub free_bikes: Option<u32>,
    pub empty_slots: Option<u32>,
    pub id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub name: String,
    timestamp: String, //TODO see how to manipulate dates
    pub extra: Option<Extra>,

    #[serde(default)]
    pub network_href: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Extra {
    pub address: Option<String>,
    pub description: Option<String>,
    // status: Option<String>,
}

pub async fn fetch_networks() -> Result<Vec<Network>, Exception> {
    #[derive(Deserialize, Serialize)]
    struct Response {
        networks: Vec<Network>,
    }
    let Response { networks } = surf::get(format!("{}{}", CITYBIKES_HOST, NETWORKS_HREF))
        .recv_json()
        .await?;
    Ok(networks)
}

pub async fn fetch_stations(network_href: &str) -> Result<Vec<Station>, Exception> {
    #[derive(Deserialize, Serialize)]
    struct Response {
        network: Network,
    }
    let Response { network } = surf::get(format!("{}{}", CITYBIKES_HOST, network_href))
        .recv_json()
        .await?;
    let Network { stations, .. } = network;
    let mut stations = stations.unwrap_or_else(|| vec![]);
    stations.iter_mut().for_each(|station| {
        station.network_href = Some(network_href.into());
    });

    Ok(stations)
}

impl Network {
    pub async fn stations(&self) -> Result<Vec<Station>, Exception> {
        let Network { href, name, .. } = self;
        let href = if let Some(href) = href {
            href
        } else {
            return Err(Box::new(BikeServiceError::InvalidBikeNetwork(
                name.to_string(),
            )));
        };
        fetch_stations(href).await
    }
}

impl Station {
    pub async fn fetch(id: &str, network_href: &str) -> Result<Self, Exception> {
        log::debug!("Fetching single station with id `{}`", &id);
        let stations = fetch_stations(network_href).await?;
        stations
            .into_iter()
            .find(|station| station.id == id)
            .ok_or(Box::new(BikeServiceError::StationNotFound(id.into())))
    }
}

pub trait Geo {
    fn location(&self) -> geoutils::Location;
}

impl Geo for Network {
    fn location(&self) -> geoutils::Location {
        let location = &self.location;
        geoutils::Location::new(location.latitude, location.longitude)
    }
}

impl Geo for Station {
    fn location(&self) -> geoutils::Location {
        geoutils::Location::new(self.latitude, self.longitude)
    }
}
