use chrono::{NaiveDate, NaiveTime};
use serde::{self, Deserialize, Deserializer};

const BASE_URL: &str = "https://ragibkl.github.io/simplesolat-data";

/// Deserialize HH:MM or HH:MM:SS time strings.
fn deserialize_time<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    NaiveTime::parse_from_str(&s, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(&s, "%H:%M:%S"))
        .map_err(serde::de::Error::custom)
}

/// Country definition from countries.yaml
#[derive(Debug, Deserialize, Clone)]
pub struct Country {
    pub code: String,
    pub name: String,
    pub geojson: String,
    pub mapping: String,
    pub shape_property: String,
}

#[derive(Debug, Deserialize)]
struct CountriesConfig {
    countries: Vec<Country>,
}

/// Zone definition from zones/{CC}.yaml
#[derive(Debug, Deserialize, Clone)]
pub struct Zone {
    pub code: String,
    pub country: String,
    pub state: String,
    pub location: String,
    pub timezone: String,
}

#[derive(Debug, Deserialize)]
struct ZonesConfig {
    zones: Vec<Zone>,
}

/// Prayer time record from prayer-times/{CC}/{zone}/{year}-{month}.json
#[derive(Debug, Deserialize)]
pub struct PrayerTimeRecord {
    pub date: NaiveDate,
    #[serde(deserialize_with = "deserialize_time")]
    pub imsak: NaiveTime,
    #[serde(deserialize_with = "deserialize_time")]
    pub fajr: NaiveTime,
    #[serde(deserialize_with = "deserialize_time")]
    pub syuruk: NaiveTime,
    #[serde(deserialize_with = "deserialize_time")]
    pub dhuhr: NaiveTime,
    #[serde(deserialize_with = "deserialize_time")]
    pub asr: NaiveTime,
    #[serde(deserialize_with = "deserialize_time")]
    pub maghrib: NaiveTime,
    #[serde(deserialize_with = "deserialize_time")]
    pub isha: NaiveTime,
}

/// Fetches countries.yaml from the data repo.
pub async fn fetch_countries(
    client: &reqwest::Client,
) -> Result<Vec<Country>, Box<dyn std::error::Error>> {
    let url = format!("{}/countries.yaml", BASE_URL);
    let text = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let config: CountriesConfig = serde_yaml::from_str(&text)?;
    Ok(config.countries)
}

/// Fetches zones/{CC}.yaml from the data repo.
pub async fn fetch_zones(
    client: &reqwest::Client,
    country_code: &str,
) -> Result<Vec<Zone>, Box<dyn std::error::Error>> {
    let url = format!("{}/zones/{}.yaml", BASE_URL, country_code);
    let response = client.get(&url).send().await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }
    let text = response.error_for_status()?.text().await?;
    let config: ZonesConfig = serde_yaml::from_str(&text)?;
    Ok(config.zones)
}

/// Fetches prayer-times/{CC}/{zone}/{year}-{month}.json from the data repo.
/// Returns empty vec on 404 (data not available yet).
pub async fn fetch_prayer_times(
    client: &reqwest::Client,
    country_code: &str,
    zone_code: &str,
    year: i32,
    month: u32,
) -> Result<Vec<PrayerTimeRecord>, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "{}/prayer-times/{}/{}/{}-{:02}.json",
        BASE_URL, country_code, zone_code, year, month
    );
    let response = client.get(&url).send().await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }
    let records: Vec<PrayerTimeRecord> = response.error_for_status()?.json().await?;
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn test_fetch_countries() {
        let countries = fetch_countries(&client()).await.unwrap();
        assert!(countries.len() >= 5);
        let my = countries.iter().find(|c| c.code == "MY").unwrap();
        assert_eq!(my.name, "Malaysia");
        assert!(my.geojson.contains("MY"));
    }

    #[tokio::test]
    async fn test_fetch_zones() {
        let zones = fetch_zones(&client(), "MY").await.unwrap();
        assert!(zones.len() >= 59);
        let sgr01 = zones.iter().find(|z| z.code == "SGR01").unwrap();
        assert_eq!(sgr01.timezone, "Asia/Kuala_Lumpur");
    }

    #[tokio::test]
    async fn test_fetch_prayer_times() {
        let records = fetch_prayer_times(&client(), "MY", "SGR01", 2026, 4).await.unwrap();
        assert_eq!(records.len(), 30); // April has 30 days
        assert_eq!(records[0].date, NaiveDate::from_ymd_opt(2026, 4, 1).unwrap());
        assert!(records[0].fajr < records[0].syuruk);
        assert!(records[0].dhuhr < records[0].asr);
    }

    #[tokio::test]
    async fn test_fetch_prayer_times_404() {
        let records = fetch_prayer_times(&client(), "MY", "SGR01", 2099, 1).await.unwrap();
        assert!(records.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_zones_unknown_country() {
        let zones = fetch_zones(&client(), "XX").await.unwrap();
        assert!(zones.is_empty());
    }
}
