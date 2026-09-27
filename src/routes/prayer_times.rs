use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{NaiveDate, NaiveDateTime, TimeDelta, TimeZone};
use serde::{Deserialize, Serialize};

use crate::{
    api::data_repo::PrayerTimeRecord,
    routes::{AppError, AppState},
};

fn datetime_to_timestamp(date: NaiveDate, time: chrono::NaiveTime, tz: chrono_tz::Tz) -> i64 {
    let naive_datetime = NaiveDateTime::new(date, time);
    // Ambiguous (DST fall-back): take the earlier instant. Nonexistent (DST
    // spring-forward gap): shift forward by an hour, as clocks do.
    tz.from_local_datetime(&naive_datetime)
        .earliest()
        .or_else(|| {
            tz.from_local_datetime(&(naive_datetime + TimeDelta::hours(1)))
                .earliest()
        })
        .expect("local time exists after skipping a DST gap")
        .timestamp()
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
    fn from_record(value: &PrayerTimeRecord, zone: &str, tz: chrono_tz::Tz) -> Self {
        Self {
            date: value.date,
            zone: zone.to_string(),
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

    // Look up zone to determine country and timezone
    let index = state.index().await?;
    let zone_info = index.zone(&zone).ok_or_else(|| AppError::NotFound(
        format!("Zone '{}' not found", zone),
    ))?;
    let tz = zone_info.tz();

    let records = state
        .store
        .prayer_times(zone_info, params.from, params.to)
        .await
        .map_err(|e| {
            tracing::error!("fetching prayer times for zone {} failed: {}", zone, e);
            AppError::BadGateway("failed to fetch prayer times from data source".to_string())
        })?;

    let response = WaktuSolatResponse {
        data: records
            .iter()
            .map(|r| WaktuSolat::from_record(r, &zone_info.code, tz))
            .collect(),
    };

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn time(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn test_timestamp_kuala_lumpur() {
        // 2026-09-27 05:54 MYT (UTC+8) = 2026-09-26 21:54 UTC
        let ts = datetime_to_timestamp(date(2026, 9, 27), time(5, 54), chrono_tz::Asia::Kuala_Lumpur);
        assert_eq!(ts, 1790459640);
    }

    #[test]
    fn test_timestamp_dst_gap_shifts_forward() {
        // Europe/Sarajevo skips 02:00-03:00 on 2026-03-29.
        let tz = chrono_tz::Europe::Sarajevo;
        assert_eq!(
            datetime_to_timestamp(date(2026, 3, 29), time(2, 30), tz),
            datetime_to_timestamp(date(2026, 3, 29), time(3, 30), tz),
        );
    }

    #[test]
    fn test_timestamp_dst_overlap_takes_earlier() {
        // Europe/Sarajevo repeats 02:00-03:00 on 2026-10-25; the first 02:30 is CEST (UTC+2).
        let ts = datetime_to_timestamp(date(2026, 10, 25), time(2, 30), chrono_tz::Europe::Sarajevo);
        assert_eq!(ts, 1792888200);
    }
}
