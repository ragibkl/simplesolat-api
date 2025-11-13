use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::{
    models::prayer_times::{SelectPrayerTime, select_prayer_times_for_zone},
    routes::{AppError, AppState},
};

fn datetime_to_timestamp(date: NaiveDate, time: NaiveTime) -> i64 {
    // Combine date + time
    let naive_datetime = NaiveDateTime::new(date, time);

    // Convert to timezone-aware datetime
    let dt = naive_datetime
        .and_local_timezone(chrono_tz::Asia::Kuala_Lumpur)
        .unwrap();

    // Get Unix timestamp
    dt.timestamp()
}

// Types matching your mobile app's expected format
#[derive(Debug, Serialize, Deserialize)]
pub struct WaktuSolat {
    pub date: NaiveDate,
    pub zone: String,
    pub imsak: i64,   // Unix timestamp
    pub fajr: i64,    // Unix timestamp
    pub syuruk: i64,  // Unix timestamp
    pub dhuhr: i64,   // Unix timestamp
    pub asr: i64,     // Unix timestamp
    pub maghrib: i64, // Unix timestamp
    pub isha: i64,    // Unix timestamp
}

impl From<&SelectPrayerTime> for WaktuSolat {
    fn from(value: &SelectPrayerTime) -> Self {
        Self {
            date: value.date,
            zone: value.zone_code.to_string(),
            imsak: datetime_to_timestamp(value.date, value.imsak),
            fajr: datetime_to_timestamp(value.date, value.fajr),
            syuruk: datetime_to_timestamp(value.date, value.syuruk),
            dhuhr: datetime_to_timestamp(value.date, value.dhuhr),
            asr: datetime_to_timestamp(value.date, value.asr),
            maghrib: datetime_to_timestamp(value.date, value.maghrib),
            isha: datetime_to_timestamp(value.date, value.isha),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaktuSolatResponse {
    pub data: Vec<WaktuSolat>,
}

// Query parameters for the prayer times endpoint
#[derive(Debug, Deserialize)]
pub struct PrayerQuery {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

pub async fn get_prayer_times(
    Path(zone): Path<String>,
    Query(params): Query<PrayerQuery>,
    State(state): State<AppState>,
) -> Result<Json<WaktuSolatResponse>, AppError> {
    tracing::info!(
        "Fetching prayer times for zone: {}, from: {}, to: {}",
        zone,
        params.from,
        params.to
    );

    // Get a connection from the pool
    let mut conn = state.db_pool.get().map_err(|e| AppError {
        message: format!("Failed to get database connection: {}", e),
    })?;

    let pts = select_prayer_times_for_zone(&mut conn, &zone, params.from, params.to);
    let response = WaktuSolatResponse {
        data: pts.iter().map(|pt| pt.into()).collect(),
    };

    Ok(Json(response))
}
