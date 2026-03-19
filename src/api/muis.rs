use chrono::{NaiveDate, NaiveTime, Timelike};
use serde::{self, Deserialize, Deserializer};

/// Parse MUIS time string (12-hour format without AM/PM) to NaiveTime.
fn parse_time(time_str: &str, is_pm: bool) -> Result<NaiveTime, String> {
    let t = NaiveTime::parse_from_str(time_str, "%H:%M")
        .map_err(|e| format!("invalid time '{}': {}", time_str, e))?;
    if is_pm && t.hour() < 12 {
        Ok(t + chrono::Duration::hours(12))
    } else {
        Ok(t)
    }
}

fn deserialize_am<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_time(&s, false).map_err(serde::de::Error::custom)
}

fn deserialize_pm<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_time(&s, true).map_err(serde::de::Error::custom)
}

/// Parsed MUIS prayer times, matching upstream field names.
/// Imsak is derived (subuh - 10 min) since MUIS doesn't provide it.
#[derive(Debug, Deserialize)]
pub struct MuisPrayerTime {
    #[serde(skip)]
    pub zone_code: String,
    #[serde(skip)]
    pub imsak: NaiveTime,
    #[serde(rename = "Date")]
    pub date: NaiveDate,
    #[serde(rename = "Subuh", deserialize_with = "deserialize_am")]
    pub subuh: NaiveTime,
    #[serde(rename = "Syuruk", deserialize_with = "deserialize_am")]
    pub syuruk: NaiveTime,
    #[serde(rename = "Zohor", deserialize_with = "deserialize_pm")]
    pub zohor: NaiveTime,
    #[serde(rename = "Asar", deserialize_with = "deserialize_pm")]
    pub asar: NaiveTime,
    #[serde(rename = "Maghrib", deserialize_with = "deserialize_pm")]
    pub maghrib: NaiveTime,
    #[serde(rename = "Isyak", deserialize_with = "deserialize_pm")]
    pub isyak: NaiveTime,
}

#[derive(Debug, Deserialize)]
struct CkanResult {
    records: Vec<MuisPrayerTime>,
}

#[derive(Debug, Deserialize)]
struct CkanResponse {
    result: CkanResult,
}

pub async fn fetch_muis_prayer_times() -> Result<Vec<MuisPrayerTime>, Box<dyn std::error::Error>> {
    let url = "https://data.gov.sg/api/action/datastore_search";

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut request = client
        .get(url)
        .header("User-Agent", "Simplesolat/1.0");

    if let Ok(api_key) = std::env::var("MUIS_API_KEY") {
        request = request.header("x-api-key", api_key);
    }

    let response = request
        .query(&[
            ("resource_id", "d_a6a206cba471fe04b62dd886ef5eaf22"),
            ("limit", "1100"),
        ])
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("MUIS API returned status: {}", response.status()).into());
    }

    let ckan: CkanResponse = response.json().await?;
    let mut prayer_times = ckan.result.records;
    for pt in &mut prayer_times {
        pt.zone_code = "SGP01".to_string();
        // MUIS doesn't provide imsak. Convention: imsak = subuh - 10 minutes.
        pt.imsak = pt.subuh - chrono::Duration::minutes(10);
    }
    Ok(prayer_times)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_subuh_am() {
        let t = parse_time("05:44", false).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(5, 44, 0).unwrap());
    }

    #[test]
    fn test_parse_time_zohor_pm_low_hour() {
        let t = parse_time("01:00", true).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(13, 0, 0).unwrap());
    }

    #[test]
    fn test_parse_time_zohor_pm_noon() {
        let t = parse_time("12:59", true).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(12, 59, 0).unwrap());
    }

    #[test]
    fn test_parse_time_isyak_pm() {
        let t = parse_time("08:02", true).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(20, 2, 0).unwrap());
    }
}
