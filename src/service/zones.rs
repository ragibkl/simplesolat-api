use diesel::PgConnection;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Zone {
    pub(crate) code: String,
    country: String,
    state: String,
    location: String,
}

#[derive(Debug, Deserialize)]
pub struct ZoneConfig {
    zones: Vec<Zone>,
}

impl From<&Zone> for crate::models::zones::UpsertZone {
    fn from(value: &Zone) -> Self {
        Self {
            zone_code: value.code.to_string(),
            country: value.country.to_string(),
            state: value.state.to_string(),
            location: value.location.to_string(),
        }
    }
}

pub fn read_zones() -> Vec<Zone> {
    let file = fs::File::open("data/zones.yaml").expect("failed to open data/zones.yaml");
    let zone_config: ZoneConfig = serde_yaml::from_reader(file).expect("failed to parse data/zones.yaml");

    zone_config.zones
}

pub fn upsert_zones_from_data(conn: &mut PgConnection) {
    let zones = read_zones();

    for zone in zones.iter() {
        if let Err(e) = crate::models::zones::upsert_zone(conn, zone.into()) {
            tracing::error!("[upsert_zones] db error for zone {}: {}", zone.code, e);
        }
    }
}
