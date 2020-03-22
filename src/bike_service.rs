use serde::{Deserialize, Serialize};
use surf::Exception;
const CITYBIKES_HOST: &str = "http://api.citybik.es";
const NETWORKS_HREF: &str = "/v2/networks";

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
    stations: Option<Vec<Station>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Station {
    pub free_bikes: Option<u32>,
    empty_slots: Option<u32>,
    id: String,
    latitude: f64,
    longitude: f64,
    pub name: String,
    timestamp: String, //TODO see how to manipulate dates
    extra: Option<Extra>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Extra {
    address: Option<String>,
    description: Option<String>,
    // status: Option<String>,
}

pub async fn fetch_networks() -> Result<Vec<Network>, Exception> {
    #[derive(Deserialize, Serialize)]
    struct Response {
        networks: Vec<Network>,
    }
    let Response { networks } = surf::get(CITYBIKES_HOST.to_owned() + NETWORKS_HREF)
        .recv_json()
        .await?;
    Ok(networks)
}

impl Network {
    pub async fn stations(&self) -> Result<Vec<Station>, Exception> {
        #[derive(Deserialize, Serialize)]
        struct Response {
            network: Network,
        }
        let Network { href, .. } = self;
        if href.is_none() {
            return Ok(vec![]);
        }
        let Response { network } = surf::get(CITYBIKES_HOST.to_string() + href.as_ref().unwrap())
            .recv_json()
            .await?;
        let Network { stations, .. } = network;
        let stations = stations.unwrap_or_else(|| vec![]);
        Ok(stations)
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

#[cfg(test)]
mod test {
    use super::*;
    // Wrote this tests just to verify if i am parsing correctly any JSON that might come from
    // citybik.es api.

    #[tokio::test]
    async fn test_fetch_network() {
        let result = fetch_networks().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fetch_station() {
        let networks = fetch_networks().await.unwrap();
        for network in networks {
            let href = network.href.as_ref().unwrap();
            // Not the best solution to await in a for loop
            let result = network.stations().await;
            if result.is_err() {
                println!("Failed on this href: '{}'", result.as_ref().err().unwrap());
            }
        }
    }
}
