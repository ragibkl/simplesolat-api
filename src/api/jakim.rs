use chrono::{NaiveDate, NaiveTime};
use serde::Deserialize;

mod jakim_date_format {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer};

    const FORMAT: &str = "%d-%b-%Y";

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

/// Parsed JAKIM prayer times, deserialized directly from upstream JSON.
#[derive(Debug, Deserialize)]
pub struct JakimPrayerTime {
    #[serde(skip)]
    pub zone_code: String,
    #[serde(with = "jakim_date_format")]
    pub date: NaiveDate,
    pub imsak: NaiveTime,
    pub fajr: NaiveTime,
    pub syuruk: NaiveTime,
    pub dhuhr: NaiveTime,
    pub asr: NaiveTime,
    pub maghrib: NaiveTime,
    pub isha: NaiveTime,
}

#[derive(Debug, Deserialize)]
struct JakimResponse {
    #[serde(rename = "prayerTime")]
    prayer_time: Vec<JakimPrayerTime>,
    status: String,
}

pub async fn fetch_jakim_prayer_times(
    zone_code: &str,
    date_start: NaiveDate,
    date_end: NaiveDate,
) -> Result<Vec<JakimPrayerTime>, Box<dyn std::error::Error>> {
    let base_url = "https://www.e-solat.gov.my/index.php";

    let date_start_str = date_start.format("%Y-%m-%d").to_string();
    let date_end_str = date_end.format("%Y-%m-%d").to_string();

    let params = [
        ("datestart", date_start_str.as_str()),
        ("dateend", date_end_str.as_str()),
    ];

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client
        .post(base_url)
        .header("User-Agent", "Simplesolat/1.0")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .query(&[
            ("r", "esolatApi/takwimsolat"),
            ("period", "duration"),
            ("zone", zone_code),
        ])
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()).into());
    }

    let jakim_response: JakimResponse = response.json().await?;

    if jakim_response.status != "OK!" {
        return Err(format!("API returned status: {}", jakim_response.status).into());
    }

    let mut prayer_times = jakim_response.prayer_time;
    for pt in &mut prayer_times {
        pt.zone_code = zone_code.to_string();
    }
    Ok(prayer_times)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_prayer_times() {
        let start = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 11, 7).unwrap();

        let result = fetch_jakim_prayer_times("SGR01", start, end).await;

        assert!(result.is_ok());
        let prayer_times = result.unwrap();
        assert!(!prayer_times.is_empty());
        assert_eq!(prayer_times.len(), 7);
    }
}
