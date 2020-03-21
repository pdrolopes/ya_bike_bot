use serde::{Deserialize, Serialize};
use surf;
const NETWORKS_HREF: &str = "http://api.citybik.es/v2/networks";

#[derive(Serialize, Deserialize, Debug)]
struct Location {
    latitude: f32,
    longitude: f32,
    city: String,
    country: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Network {
    href: String,
    location: Location,
    stations: Option<Vec<Station>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Station {
    free_bikes: u32,
    empty_slots: u32,
    id: String,
    latitude: f32,
    longitude: f32,
    name: String,
    timestamp: String,
    extra: Extra,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Extra {
    address: String,
    description: String,
    status: String,
}

type AsyncError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub async fn fetch_networks() -> Result<Vec<Network>, AsyncError> {
    #[derive(Deserialize, Serialize)]
    struct Response {
        networks: Vec<Network>,
    }
    let Response { networks } = surf::get(NETWORKS_HREF).recv_json().await?;
    Ok(networks)
}

impl Network {
    async fn stations(&self) -> Result<Vec<Station>, AsyncError> {
        #[derive(Deserialize, Serialize)]
        struct Response {
            network: Network,
        }
        let Response { network } = surf::get(NETWORKS_HREF).recv_json().await?;
        let Network { stations, .. } = network;
        let stations = if let Some(stations) = stations {
            stations
        } else {
            vec![]
        };
        Ok(stations)
    }
}
