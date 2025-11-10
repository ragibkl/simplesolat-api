use std::fs;
use diesel::PgConnection;
use serde::Deserialize;


#[derive(Debug, Deserialize)]
pub struct Zone {
    zone_code: String,
    state: String,
    location: String,
}

impl From<&Zone> for crate::models::zones::UpsertZone {
    fn from(value: &Zone) -> Self {
        Self {
            zone_code: value.zone_code.to_string(),
            state: value.state.to_string(),
            location: value.location.to_string(),
        }
    }
}

pub fn read_zones() -> Vec<Zone> {
    let file = fs::File::open("data/zones.yaml").unwrap();
    let zones: Vec<Zone> = serde_yaml::from_reader(file).unwrap();

    zones
}

pub fn upsert_zones_from_data(conn: &mut PgConnection) {
    let zones = read_zones();

    for zone in zones.iter() {
        crate::models::zones::upsert_zone(conn, zone.into());
    }
}
