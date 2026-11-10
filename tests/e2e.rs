//! E2E tests for the simplesolat API.
//!
//! These tests require a running instance of the API at localhost:3000,
//! which reads from the live simplesolat-data CDN. Run with:
//!   docker compose up -d --build   (or: cargo run)
//!   cargo test --test e2e
//! Set API_URL to test another instance (default http://localhost:3000).

use std::sync::LazyLock;

use serde::Deserialize;

static BASE_URL: LazyLock<String> = LazyLock::new(|| {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
});

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
    let resp = reqwest::get(format!("{}/health", *BASE_URL))
        .await
        .expect("Failed to connect to API");

    assert!(resp.status().is_success());
    let body: HealthResponse = resp.json().await.unwrap();
    assert_eq!(body.status, "ok");
    assert_eq!(body.service, "simplesolat-api");
}

#[tokio::test]
async fn test_zones_returns_all_zones_with_country() {
    let resp = reqwest::get(format!("{}/zones", *BASE_URL))
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
    let resp = reqwest::get(format!("{}/zones", *BASE_URL))
        .await
        .expect("Failed to connect to API");

    let body: ZonesResponse = resp.json().await.unwrap();
    let sgp = body.data.iter().find(|z| z.zone == "SGP01");

    assert!(sgp.is_some(), "SGP01 not found in zones");
    let sgp = sgp.unwrap();
    assert_eq!(sgp.country, "SG");
    assert_eq!(sgp.state, "Singapore");
    assert_eq!(sgp.location, "Singapore");
}

#[tokio::test]
async fn test_zones_my_zones_have_country_my() {
    let resp = reqwest::get(format!("{}/zones", *BASE_URL))
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
        *BASE_URL
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
        *BASE_URL
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
        *BASE_URL
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
async fn test_prayer_times_unknown_zone_returns_404() {
    let resp = reqwest::get(format!(
        "{}/prayer-times/by-zone/FAKE99?from=2026-01-01&to=2026-01-01",
        *BASE_URL
    ))
    .await
    .expect("Failed to connect to API");

    assert_eq!(resp.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_prayer_times_unpublished_months_return_empty() {
    let resp = reqwest::get(format!(
        "{}/prayer-times/by-zone/SGR01?from=2099-01-01&to=2099-01-31",
        *BASE_URL
    ))
    .await
    .expect("Failed to connect to API");

    assert!(resp.status().is_success());
    let body: WaktuSolatResponse = resp.json().await.unwrap();
    assert!(body.data.is_empty(), "Months not published yet should return empty data");
}

// v2

async fn v2(method: reqwest::Method, path: &str, body: Option<serde_json::Value>) -> (u16, serde_json::Value) {
    let mut req = reqwest::Client::new().request(method, format!("{}{}", *BASE_URL, path));
    if let Some(body) = body {
        req = req.json(&body);
    }
    let resp = req.send().await.expect("Failed to connect to API");
    let status = resp.status().as_u16();
    (status, resp.json().await.unwrap())
}

async fn v2_get(path: &str) -> (u16, serde_json::Value) {
    v2(reqwest::Method::GET, path, None).await
}

#[tokio::test]
async fn test_v2_zone_by_location() {
    let (status, body) = v2(
        reqwest::Method::POST,
        "/v2/zones/by-location",
        Some(serde_json::json!({"lat": 3.139, "lng": 101.687})),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["zone"], "WLY01");
    assert_eq!(body["data"]["timezone"], "Asia/Kuala_Lumpur");
    assert_eq!(body["data"]["authority"], "JAKIM");
}

#[tokio::test]
async fn test_v2_coastal_points_take_the_nearest_district() {
    // Just off the district maps: Kukup's stilt village and Pulau Perhentian.
    for (lat, lng, zone) in [(1.325, 103.44, "JHR03"), (5.91, 102.74, "TRG02")] {
        let (status, body) = v2(
            reqwest::Method::POST,
            "/v2/zones/by-location",
            Some(serde_json::json!({"lat": lat, "lng": lng})),
        )
        .await;
        assert_eq!(status, 200, "{lat},{lng}");
        assert_eq!(body["data"]["zone"], zone, "{lat},{lng}");
    }
}

#[tokio::test]
async fn test_v2_no_official_zone() {
    let (status, body) = v2(
        reqwest::Method::POST,
        "/v2/zones/by-location",
        Some(serde_json::json!({"lat": 0.0, "lng": 0.0})),
    )
    .await;
    assert_eq!(status, 404);
    assert_eq!(body["error"]["code"], "no_official_zone");
}

#[tokio::test]
async fn test_v2_day_has_local_times_and_dhuha() {
    let (status, body) = v2_get("/v2/zones/WLY01/prayer-times/day/2026-10-27").await;
    assert_eq!(status, 200);
    assert_eq!(body["zone"], "WLY01");
    assert_eq!(body["data"]["date"], "2026-10-27");
    assert_eq!(body["data"]["dhuhr"], "13:00");
    assert!(body["data"]["dhuha"].is_string());
}

#[tokio::test]
async fn test_v2_month_and_year() {
    let (_, month) = v2_get("/v2/zones/SGP01/prayer-times/month/2026-02").await;
    assert_eq!(month["data"].as_array().unwrap().len(), 28);
    assert!(month["data"][0].get("dhuha").is_none(), "MUIS publishes no Dhuha");
    let (_, year) = v2_get("/v2/zones/BRN01/prayer-times/year/2026").await;
    assert_eq!(year["data"].as_array().unwrap().len(), 365);
}

#[tokio::test]
async fn test_v2_countries_and_zones() {
    let (_, countries) = v2_get("/v2/countries").await;
    assert!(countries["data"].as_array().unwrap().iter().any(|c| c["code"] == "MY"));
    let (_, zones) = v2_get("/v2/countries/BN/zones").await;
    assert_eq!(zones["data"].as_array().unwrap().len(), 4);
    let (status, zone) = v2_get("/v2/zones/SGP01").await;
    assert_eq!(status, 200);
    assert_eq!(zone["data"]["country"], "SG");
}

#[tokio::test]
async fn test_v2_errors() {
    let (status, body) = v2_get("/v2/zones/WLY01/prayer-times/month/2026-13").await;
    assert_eq!((status, body["error"]["code"].as_str()), (400, Some("invalid_month")));
    let (status, body) = v2_get("/v2/zones/NOPE/prayer-times/year/2026").await;
    assert_eq!((status, body["error"]["code"].as_str()), (404, Some("zone_not_found")));
    let (status, body) = v2_get("/v2/countries/XX").await;
    assert_eq!((status, body["error"]["code"].as_str()), (404, Some("country_not_found")));
}

async fn times_at(body: serde_json::Value) -> (u16, serde_json::Value) {
    v2(reqwest::Method::POST, "/v2/prayer-times/by-location", Some(body)).await
}

#[tokio::test]
async fn test_v2_times_by_location_official() {
    let (status, body) =
        times_at(serde_json::json!({"lat": 3.139, "lng": 101.687, "date": "2026-11-10"})).await;
    assert_eq!(status, 200);
    assert_eq!(body["source"], "official");
    assert_eq!(body["zone"], "WLY01");
    assert_eq!(body["authority"], "JAKIM");
    assert_eq!(body["country_name"], "Malaysia");
    let day = &body["data"][0];
    assert_eq!(day["date"], "2026-11-10");
    assert_eq!(day["utc_offset"], "+08:00");
    // Same times as the zone endpoint.
    let (_, zone_day) = v2_get("/v2/zones/WLY01/prayer-times/day/2026-11-10").await;
    for k in ["imsak", "fajr", "syuruk", "dhuha", "dhuhr", "asr", "maghrib", "isha"] {
        assert_eq!(day[k], zone_day["data"][k], "{k}");
    }
}

#[tokio::test]
async fn test_v2_times_by_location_calculated_like_the_app() {
    // Auckland: no official timetable. Expected times from adhan-js (the
    // app's calculator) with Muslim World League, Shafi'i Asr.
    let (status, body) =
        times_at(serde_json::json!({"lat": -36.8485, "lng": 174.7633, "date": "2026-11-10"})).await;
    assert_eq!(status, 200);
    assert_eq!(body["source"], "calculated");
    assert_eq!(body["zone"], serde_json::Value::Null);
    assert_eq!(body["country"], "NZ");
    assert_eq!(body["timezone"], "Pacific/Auckland");
    assert_eq!(body["method"]["code"], "MuslimWorldLeague");
    let day = &body["data"][0];
    assert_eq!(day["utc_offset"], "+13:00");
    let got: Vec<&str> = ["imsak", "fajr", "syuruk", "dhuhr", "asr", "maghrib", "isha"]
        .iter()
        .map(|k| day[*k].as_str().unwrap())
        .collect();
    assert_eq!(got, ["04:18", "04:28", "06:07", "13:06", "16:53", "20:03", "21:36"]);
    assert!(day.get("dhuha").is_none());
}

#[tokio::test]
async fn test_v2_times_by_location_country_method_and_month() {
    // Mecca is in an official zone (Umm al-Qura); London is calculated, a
    // whole month, with the default method.
    let (status, body) =
        times_at(serde_json::json!({"lat": 51.5074, "lng": -0.1278, "month": "2026-11"})).await;
    assert_eq!(status, 200);
    assert_eq!(body["source"], "calculated");
    assert_eq!(body["country"], "GB");
    assert_eq!(body["data"].as_array().unwrap().len(), 30);
    assert_eq!(body["data"][9]["isha"], "18:07");
}

#[tokio::test]
async fn test_v2_times_by_location_bad_requests() {
    let (status, body) = times_at(serde_json::json!({"lat": 3.1, "lng": 101.6})).await;
    assert_eq!((status, body["error"]["code"].as_str()), (400, Some("invalid_request")));
    let (status, body) =
        times_at(serde_json::json!({"lat": 95.0, "lng": 101.6, "date": "2026-11-10"})).await;
    assert_eq!((status, body["error"]["code"].as_str()), (400, Some("invalid_location")));
}

#[tokio::test]
async fn test_v2_methods() {
    let (status, body) = v2_get("/v2/methods").await;
    assert_eq!(status, 200);
    assert_eq!(body["default"], "MuslimWorldLeague");
    assert_eq!(body["countries"]["SA"], "UmmAlQura");
}
