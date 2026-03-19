use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::{
    models::{
        prayer_times::{SelectPrayerTime, select_prayer_times_for_zone},
        zones::select_zone_by_code,
    },
    routes::{AppError, AppState},
};

fn datetime_to_timestamp(date: NaiveDate, time: NaiveTime, tz: chrono_tz::Tz) -> i64 {
    let naive_datetime = NaiveDateTime::new(date, time);
    let dt = naive_datetime.and_local_timezone(tz).unwrap();
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

impl WaktuSolat {
    fn from_prayer_time(value: &SelectPrayerTime, tz: chrono_tz::Tz) -> Self {
        Self {
            date: value.date,
            zone: value.zone_code.to_string(),
            imsak: datetime_to_timestamp(value.date, value.imsak, tz),
            fajr: datetime_to_timestamp(value.date, value.fajr, tz),
            syuruk: datetime_to_timestamp(value.date, value.syuruk, tz),
            dhuhr: datetime_to_timestamp(value.date, value.dhuhr, tz),
            asr: datetime_to_timestamp(value.date, value.asr, tz),
            maghrib: datetime_to_timestamp(value.date, value.maghrib, tz),
            isha: datetime_to_timestamp(value.date, value.isha, tz),
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
    // Validate date range
    if params.from > params.to {
        return Err(AppError::BadRequest(
            "'from' date must be before or equal to 'to' date".to_string(),
        ));
    }
    let max_days = 750; // >2 years
    if (params.to - params.from).num_days() > max_days {
        return Err(AppError::BadRequest(
            format!("Date range cannot exceed {} days", max_days),
        ));
    }

    tracing::info!(
        "fetching prayer times for zone {}, from {} to {}",
        zone,
        params.from,
        params.to
    );

    // Get a connection from the pool
    let mut conn = state.db_pool.get()?;

    // Look up zone to determine timezone
    let zone_info = select_zone_by_code(&mut conn, &zone)?;
    let zone_info = zone_info.ok_or_else(|| AppError::NotFound(
        format!("Zone '{}' not found", zone),
    ))?;
    let tz = zone_info.timezone();

    let pts = select_prayer_times_for_zone(&mut conn, &zone, params.from, params.to)?;
    let response = WaktuSolatResponse {
        data: pts.iter().map(|pt| WaktuSolat::from_prayer_time(pt, tz)).collect(),
    };

    Ok(Json(response))
}
