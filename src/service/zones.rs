use diesel::PgConnection;
use serde::Deserialize;
use std::fs;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Zone {
    code: String,
    state: String,
    location: String,
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize)]
pub struct ZoneConfig {
    zones: Vec<Zone>,
}

impl From<&Zone> for crate::models::zones::UpsertZone {
    fn from(value: &Zone) -> Self {
        Self {
            zone_code: value.code.to_string(),
            state: value.state.to_string(),
            location: value.location.to_string(),
        }
    }
}

pub fn read_zones() -> Vec<Zone> {
    let file = fs::File::open("data/zones.yaml").unwrap();
    let zone_config: ZoneConfig = serde_yaml::from_reader(file).unwrap();

    zone_config.zones
}

pub fn upsert_zones_from_data(conn: &mut PgConnection) {
    let zones = read_zones();

    for zone in zones.iter() {
        crate::models::zones::upsert_zone(conn, zone.into());
    }
}
