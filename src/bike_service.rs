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
    href: String,
    location: Location,
    stations: Option<Vec<Station>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Station {
    free_bikes: u32,
    empty_slots: u32,
    id: String,
    latitude: f64,
    longitude: f64,
    pub name: String,
    timestamp: String, //TODO see how to manipulate dates
    extra: Extra,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Extra {
    address: String,
    description: String,
    status: String,
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
        let Response { network } = surf::get(CITYBIKES_HOST.to_string() + &href)
            .recv_json()
            .await?;
        let Network { stations, .. } = network;
        let stations = stations.unwrap_or(vec![]);
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

    #[tokio::test]
    async fn test_networks() {
        let result = fetch_networks().await;
        println!("{:#?}", result);
        assert!(result.is_ok());
    }
}
