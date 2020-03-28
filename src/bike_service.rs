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
    pub name: String,
    stations: Option<Vec<Station>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Station {
    pub free_bikes: Option<u32>,
    pub empty_slots: Option<u32>,
    id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub name: String,
    timestamp: String, //TODO see how to manipulate dates
    pub extra: Option<Extra>,
}

#[derive(Serialize, Deserialize, Debug)]
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
        let Network { href, name, .. } = self;
        if href.is_none() {
            log::info!("Network name:'{}' has no href", name);
            return Ok(vec![]);
        }
        let href = href.as_ref().unwrap();
        let Response { network } = surf::get(CITYBIKES_HOST.to_string() + href)
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
