//! /v2: local "HH:MM" times, zone lookup by GPS location, and one URL per
//! kind of request (no optional query parameters).
//!
//!   POST /v2/zones/by-location                    {"lat": 3.139, "lng": 101.687}
//!   GET  /v2/zones
//!   GET  /v2/zones/{zone}
//!   GET  /v2/zones/{zone}/prayer-times/day/{YYYY-MM-DD}
//!   GET  /v2/zones/{zone}/prayer-times/month/{YYYY-MM}
//!   GET  /v2/zones/{zone}/prayer-times/year/{YYYY}
//!   GET  /v2/countries
//!   GET  /v2/countries/{country}
//!   GET  /v2/countries/{country}/zones
//!
//! Errors are {"error": {"code": "...", "message": "..."}}.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{Datelike, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::{
    api::data_repo::{self, PrayerTimeRecord},
    routes::AppState,
    store::Index,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/zones", get(get_zones))
        .route("/zones/by-location", post(get_zone_by_location))
        .route("/zones/{zone}", get(get_zone))
        .route("/zones/{zone}/prayer-times/day/{date}", get(get_day))
        .route("/zones/{zone}/prayer-times/month/{month}", get(get_month))
        .route("/zones/{zone}/prayer-times/year/{year}", get(get_year))
        .route("/countries", get(get_countries))
        .route("/countries/{country}", get(get_country))
        .route("/countries/{country}/zones", get(get_country_zones))
}

// Errors

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }

    fn data_source() -> Self {
        Self::new(
            StatusCode::BAD_GATEWAY,
            "data_source_unavailable",
            "couldn't load data from simplesolat-data; try again shortly",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let body = serde_json::json!({ "error": { "code": self.code, "message": self.message } });
        (self.status, Json(body)).into_response()
    }
}

// Responses

#[derive(Debug, Serialize, Deserialize)]
pub struct Country {
    pub code: String,
    pub name: String,
    /// The authority whose timetable this is, e.g. "JAKIM".
    pub authority: String,
}

impl From<&data_repo::Country> for Country {
    fn from(c: &data_repo::Country) -> Self {
        Self {
            code: c.code.clone(),
            name: c.name.clone(),
            authority: c.source.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Zone {
    pub zone: String,
    pub country: String,
    pub state: String,
    pub location: String,
    pub timezone: String,
    pub authority: String,
}

impl Zone {
    fn new(zone: &data_repo::Zone, index: &Index) -> Self {
        Self {
            zone: zone.code.clone(),
            country: zone.country.clone(),
            state: zone.state.clone(),
            location: zone.location.clone(),
            timezone: zone.timezone.clone(),
            authority: authority(index, &zone.country),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocatedZone {
    #[serde(flatten)]
    pub zone: Zone,
    /// The district the location is in, from the map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub district: Option<String>,
}

/// One day's times, local "HH:MM" in the zone's timezone.
#[derive(Debug, Serialize, Deserialize)]
pub struct PrayerTimes {
    pub date: NaiveDate,
    pub imsak: String,
    pub fajr: String,
    pub syuruk: String,
    /// Only where the authority publishes it (JAKIM, KHEU, Kemenag).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dhuha: Option<String>,
    pub dhuhr: String,
    pub asr: String,
    pub maghrib: String,
    pub isha: String,
}

fn hhmm(t: NaiveTime) -> String {
    t.format("%H:%M").to_string()
}

impl From<&PrayerTimeRecord> for PrayerTimes {
    fn from(r: &PrayerTimeRecord) -> Self {
        Self {
            date: r.date,
            imsak: hhmm(r.imsak),
            fajr: hhmm(r.fajr),
            syuruk: hhmm(r.syuruk),
            dhuha: r.dhuha.map(hhmm),
            dhuhr: hhmm(r.dhuhr),
            asr: hhmm(r.asr),
            maghrib: hhmm(r.maghrib),
            isha: hhmm(r.isha),
        }
    }
}

/// `data` is one object for a day, a list for a month or a year.
#[derive(Debug, Serialize, Deserialize)]
pub struct ZonePrayerTimes<T> {
    pub zone: String,
    pub timezone: String,
    pub authority: String,
    pub data: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct List<T> {
    pub data: Vec<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct One<T> {
    pub data: T,
}

// Helpers

fn authority(index: &Index, country: &str) -> String {
    index
        .countries
        .iter()
        .find(|c| c.code == country)
        .map(|c| c.source.clone())
        .unwrap_or_default()
}

async fn index(state: &AppState) -> Result<Arc<Index>, ApiError> {
    state.store.index().await.map_err(|e| {
        tracing::error!("failed to load zone index: {}", e);
        ApiError::data_source()
    })
}

fn find_zone<'a>(index: &'a Index, code: &str) -> Result<&'a data_repo::Zone, ApiError> {
    index.zone(code).ok_or_else(|| {
        ApiError::not_found(
            "zone_not_found",
            format!("no zone '{}'; see /v2/zones", code),
        )
    })
}

fn find_country<'a>(index: &'a Index, code: &str) -> Result<&'a data_repo::Country, ApiError> {
    index
        .countries
        .iter()
        .find(|c| c.code == code)
        .ok_or_else(|| {
            ApiError::not_found(
                "country_not_found",
                format!("no country '{}'; see /v2/countries", code),
            )
        })
}

async fn times(
    state: &AppState,
    zone: &data_repo::Zone,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<PrayerTimes>, ApiError> {
    let records = state
        .store
        .prayer_times(zone, from, to)
        .await
        .map_err(|e| {
            tracing::error!("fetching prayer times for zone {} failed: {}", zone.code, e);
            ApiError::data_source()
        })?;
    if records.is_empty() {
        return Err(ApiError::not_found(
            "no_prayer_times",
            format!(
                "no prayer times for {} from {} to {} yet",
                zone.code, from, to
            ),
        ));
    }
    Ok(records.iter().map(PrayerTimes::from).collect())
}

fn parse_date(s: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| {
        ApiError::bad_request("invalid_date", "date must be YYYY-MM-DD, e.g. 2026-10-25")
    })
}

/// First and last day of a "YYYY-MM" month.
fn parse_month(s: &str) -> Result<(NaiveDate, NaiveDate), ApiError> {
    let bad = || ApiError::bad_request("invalid_month", "month must be YYYY-MM, e.g. 2026-10");
    let (y, m) = s.split_once('-').ok_or_else(bad)?;
    if y.len() != 4 || m.len() != 2 {
        return Err(bad());
    }
    let first = NaiveDate::from_ymd_opt(
        y.parse().map_err(|_| bad())?,
        m.parse().map_err(|_| bad())?,
        1,
    )
    .ok_or_else(bad)?;
    let next = if first.month() == 12 {
        NaiveDate::from_ymd_opt(first.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1)
    }
    .ok_or_else(bad)?;
    Ok((first, next.pred_opt().ok_or_else(bad)?))
}

fn parse_year(s: &str) -> Result<(NaiveDate, NaiveDate), ApiError> {
    let bad = || ApiError::bad_request("invalid_year", "year must be YYYY, e.g. 2026");
    if s.len() != 4 {
        return Err(bad());
    }
    let year: i32 = s.parse().map_err(|_| bad())?;
    Ok((
        NaiveDate::from_ymd_opt(year, 1, 1).ok_or_else(bad)?,
        NaiveDate::from_ymd_opt(year, 12, 31).ok_or_else(bad)?,
    ))
}

// Handlers

async fn get_countries(State(state): State<AppState>) -> Result<Json<List<Country>>, ApiError> {
    let index = index(&state).await?;
    Ok(Json(List {
        data: index.countries.iter().map(Country::from).collect(),
    }))
}

async fn get_country(
    State(state): State<AppState>,
    Path(country): Path<String>,
) -> Result<Json<One<Country>>, ApiError> {
    let index = index(&state).await?;
    Ok(Json(One {
        data: find_country(&index, &country)?.into(),
    }))
}

async fn get_country_zones(
    State(state): State<AppState>,
    Path(country): Path<String>,
) -> Result<Json<List<Zone>>, ApiError> {
    let index = index(&state).await?;
    find_country(&index, &country)?;
    Ok(Json(List {
        data: index
            .zones
            .iter()
            .filter(|z| z.country == country)
            .map(|z| Zone::new(z, &index))
            .collect(),
    }))
}

async fn get_zones(State(state): State<AppState>) -> Result<Json<List<Zone>>, ApiError> {
    let index = index(&state).await?;
    Ok(Json(List {
        data: index.zones.iter().map(|z| Zone::new(z, &index)).collect(),
    }))
}

async fn get_zone(
    State(state): State<AppState>,
    Path(zone): Path<String>,
) -> Result<Json<One<Zone>>, ApiError> {
    let index = index(&state).await?;
    Ok(Json(One {
        data: Zone::new(find_zone(&index, &zone)?, &index),
    }))
}

#[derive(Debug, Deserialize)]
pub struct Location {
    pub lat: f64,
    pub lng: f64,
}

/// POST so the location stays out of URLs, caches and access logs; it is
/// never logged here either.
async fn get_zone_by_location(
    State(state): State<AppState>,
    body: Result<Json<Location>, JsonRejection>,
) -> Result<Json<One<LocatedZone>>, ApiError> {
    let Json(Location { lat, lng }) = body.map_err(|_| {
        ApiError::bad_request(
            "invalid_location",
            "send JSON like {\"lat\": 3.139, \"lng\": 101.687}",
        )
    })?;
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lng) {
        return Err(ApiError::bad_request(
            "invalid_location",
            "lat must be -90 to 90 and lng -180 to 180",
        ));
    }

    let index = index(&state).await?;
    let maps = state.store.maps().await.map_err(|e| {
        tracing::error!("failed to load maps: {}", e);
        ApiError::data_source()
    })?;
    let located = maps.locate(lat, lng).ok_or_else(|| {
        ApiError::not_found(
            "no_official_zone",
            "this location isn't in an official zone; see /v2/countries for the countries covered",
        )
    })?;
    let zone = find_zone(&index, &located.zone)?;
    Ok(Json(One {
        data: LocatedZone {
            zone: Zone {
                state: located.state,
                ..Zone::new(zone, &index)
            },
            district: located.district,
        },
    }))
}

async fn get_day(
    State(state): State<AppState>,
    Path((zone, date)): Path<(String, String)>,
) -> Result<Json<ZonePrayerTimes<PrayerTimes>>, ApiError> {
    let date = parse_date(&date)?;
    let index = index(&state).await?;
    let zone = find_zone(&index, &zone)?;
    let mut days = times(&state, zone, date, date).await?;
    Ok(Json(ZonePrayerTimes {
        zone: zone.code.clone(),
        timezone: zone.timezone.clone(),
        authority: authority(&index, &zone.country),
        data: days.remove(0),
    }))
}

async fn get_range(
    state: &AppState,
    zone: &str,
    (from, to): (NaiveDate, NaiveDate),
) -> Result<Json<ZonePrayerTimes<Vec<PrayerTimes>>>, ApiError> {
    let index = index(state).await?;
    let zone = find_zone(&index, zone)?;
    let data = times(state, zone, from, to).await?;
    Ok(Json(ZonePrayerTimes {
        zone: zone.code.clone(),
        timezone: zone.timezone.clone(),
        authority: authority(&index, &zone.country),
        data,
    }))
}

async fn get_month(
    State(state): State<AppState>,
    Path((zone, month)): Path<(String, String)>,
) -> Result<Json<ZonePrayerTimes<Vec<PrayerTimes>>>, ApiError> {
    get_range(&state, &zone, parse_month(&month)?).await
}

async fn get_year(
    State(state): State<AppState>,
    Path((zone, year)): Path<(String, String)>,
) -> Result<Json<ZonePrayerTimes<Vec<PrayerTimes>>>, ApiError> {
    get_range(&state, &zone, parse_year(&year)?).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn test_parse_month() {
        assert_eq!(
            parse_month("2026-02").unwrap(),
            (d(2026, 2, 1), d(2026, 2, 28))
        );
        assert_eq!(
            parse_month("2026-12").unwrap(),
            (d(2026, 12, 1), d(2026, 12, 31))
        );
        assert_eq!(parse_month("2028-02").unwrap().1, d(2028, 2, 29));
        for bad in [
            "2026-13",
            "2026-1",
            "26-10",
            "2026/10",
            "2026-10-01",
            "abcd-ef",
        ] {
            assert_eq!(
                parse_month(bad).unwrap_err().code,
                "invalid_month",
                "{}",
                bad
            );
        }
    }

    #[test]
    fn test_parse_year_and_date() {
        assert_eq!(
            parse_year("2026").unwrap(),
            (d(2026, 1, 1), d(2026, 12, 31))
        );
        assert_eq!(parse_year("26").unwrap_err().code, "invalid_year");
        assert_eq!(parse_date("2026-10-25").unwrap(), d(2026, 10, 25));
        assert_eq!(parse_date("2026-02-30").unwrap_err().code, "invalid_date");
    }
}
