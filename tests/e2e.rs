//! E2E tests for the simplesolat API.
//!
//! These tests require a running instance of the API at localhost:3000
//! with synced data. Run with:
//!   docker compose up -d
//!   docker compose exec simplesolat-api sync
//!   cargo test --test e2e

use serde::Deserialize;

const BASE_URL: &str = "http://localhost:3000";

#[derive(Debug, Deserialize)]
struct HealthResponse {
    status: String,
    service: String,
}

#[derive(Debug, Deserialize)]
struct Zone {
    zone: String,
    country: String,
    state: String,
    location: String,
}

#[derive(Debug, Deserialize)]
struct ZonesResponse {
    data: Vec<Zone>,
}

#[derive(Debug, Deserialize)]
struct WaktuSolat {
    date: String,
    zone: String,
    imsak: i64,
    fajr: i64,
    syuruk: i64,
    dhuhr: i64,
    asr: i64,
    maghrib: i64,
    isha: i64,
}

#[derive(Debug, Deserialize)]
struct WaktuSolatResponse {
    data: Vec<WaktuSolat>,
}

#[tokio::test]
async fn test_health_check() {
    let resp = reqwest::get(format!("{}/health", BASE_URL))
        .await
        .expect("Failed to connect to API");

    assert!(resp.status().is_success());
    let body: HealthResponse = resp.json().await.unwrap();
    assert_eq!(body.status, "ok");
    assert_eq!(body.service, "simplesolat-api");
}

#[tokio::test]
async fn test_zones_returns_all_zones_with_country() {
    let resp = reqwest::get(format!("{}/zones", BASE_URL))
        .await
        .expect("Failed to connect to API");

    assert!(resp.status().is_success());
    let body: ZonesResponse = resp.json().await.unwrap();

    // Should have at least 61 zones (60 MY + 1 SG)
    assert!(body.data.len() >= 61, "Expected >= 61 zones, got {}", body.data.len());

    // All zones should have a country field
    for zone in &body.data {
        assert!(!zone.country.is_empty(), "Zone {} missing country", zone.zone);
    }
}

#[tokio::test]
async fn test_zones_contains_sgp01() {
    let resp = reqwest::get(format!("{}/zones", BASE_URL))
        .await
        .expect("Failed to connect to API");

    let body: ZonesResponse = resp.json().await.unwrap();
    let sgp = body.data.iter().find(|z| z.zone == "SGP01");

    assert!(sgp.is_some(), "SGP01 not found in zones");
    let sgp = sgp.unwrap();
    assert_eq!(sgp.country, "SG");
    assert_eq!(sgp.state, "Singapore");
    assert_eq!(sgp.location, "Seluruh Singapura");
}

#[tokio::test]
async fn test_zones_my_zones_have_country_my() {
    let resp = reqwest::get(format!("{}/zones", BASE_URL))
        .await
        .expect("Failed to connect to API");

    let body: ZonesResponse = resp.json().await.unwrap();
    let sgr01 = body.data.iter().find(|z| z.zone == "SGR01").unwrap();
    assert_eq!(sgr01.country, "MY");
}

#[tokio::test]
async fn test_prayer_times_sgr01() {
    let resp = reqwest::get(format!(
        "{}/prayer-times/by-zone/SGR01?from=2026-01-01&to=2026-01-01",
        BASE_URL
    ))
    .await
    .expect("Failed to connect to API");

    assert!(resp.status().is_success());
    let body: WaktuSolatResponse = resp.json().await.unwrap();
    assert_eq!(body.data.len(), 1);

    let pt = &body.data[0];
    assert_eq!(pt.zone, "SGR01");
    assert_eq!(pt.date, "2026-01-01");

    // Prayer times should be in chronological order
    assert!(pt.imsak < pt.fajr, "imsak should be before fajr");
    assert!(pt.fajr < pt.syuruk, "fajr should be before syuruk");
    assert!(pt.syuruk < pt.dhuhr, "syuruk should be before dhuhr");
    assert!(pt.dhuhr < pt.asr, "dhuhr should be before asr");
    assert!(pt.asr < pt.maghrib, "asr should be before maghrib");
    assert!(pt.maghrib < pt.isha, "maghrib should be before isha");
}

#[tokio::test]
async fn test_prayer_times_sgp01() {
    let resp = reqwest::get(format!(
        "{}/prayer-times/by-zone/SGP01?from=2026-01-01&to=2026-01-01",
        BASE_URL
    ))
    .await
    .expect("Failed to connect to API");

    assert!(resp.status().is_success());
    let body: WaktuSolatResponse = resp.json().await.unwrap();
    assert_eq!(body.data.len(), 1);

    let pt = &body.data[0];
    assert_eq!(pt.zone, "SGP01");
    assert_eq!(pt.date, "2026-01-01");

    // Prayer times should be in chronological order
    assert!(pt.imsak < pt.fajr, "imsak should be before fajr");
    assert!(pt.fajr < pt.syuruk, "fajr should be before syuruk");
    assert!(pt.syuruk < pt.dhuhr, "syuruk should be before dhuhr");
    assert!(pt.dhuhr < pt.asr, "dhuhr should be before asr");
    assert!(pt.asr < pt.maghrib, "asr should be before maghrib");
    assert!(pt.maghrib < pt.isha, "maghrib should be before isha");
}

#[tokio::test]
async fn test_prayer_times_sgp01_date_range() {
    let resp = reqwest::get(format!(
        "{}/prayer-times/by-zone/SGP01?from=2026-01-01&to=2026-01-31",
        BASE_URL
    ))
    .await
    .expect("Failed to connect to API");

    assert!(resp.status().is_success());
    let body: WaktuSolatResponse = resp.json().await.unwrap();
    assert_eq!(body.data.len(), 31, "Should have 31 days of data for Jan 2026");

    // All entries should be for SGP01
    for pt in &body.data {
        assert_eq!(pt.zone, "SGP01");
    }
}

#[tokio::test]
async fn test_prayer_times_unknown_zone_returns_empty() {
    let resp = reqwest::get(format!(
        "{}/prayer-times/by-zone/FAKE99?from=2026-01-01&to=2026-01-01",
        BASE_URL
    ))
    .await
    .expect("Failed to connect to API");

    assert!(resp.status().is_success());
    let body: WaktuSolatResponse = resp.json().await.unwrap();
    assert!(body.data.is_empty(), "Unknown zone should return empty data");
}
