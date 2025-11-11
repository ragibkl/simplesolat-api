use chrono::NaiveDate;
use reqwest;
use serde::{Deserialize, Serialize};

/// Represents a single day's prayer times from Jakim API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrayerTime {
    pub hijri: String,
    pub date: String,
    pub day: String,
    pub imsak: String,
    pub fajr: String,
    pub syuruk: String,
    pub dhuhr: String,
    pub asr: String,
    pub maghrib: String,
    pub isha: String,
}

/// Response wrapper from Jakim API
#[derive(Debug, Deserialize)]
struct JakimResponse {
    #[serde(rename = "prayerTime")]
    prayer_time: Vec<PrayerTime>,
    status: String,
    // zone: String,
}

/// Fetches prayer times from Jakim e-solat API
///
/// # Arguments
/// * `zone_code` - Malaysian prayer zone code (e.g., "SGR01", "WLY01")
/// * `date_start` - Start date for the range
/// * `date_end` - End date for the range
///
/// # Returns
/// * `Result<Vec<PrayerTime>, Box<dyn std::error::Error>>` - Vector of prayer times or error
///
/// # Example
/// ```
/// use chrono::NaiveDate;
///
/// let start = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();
/// let end = NaiveDate::from_ymd_opt(2025, 11, 30).unwrap();
/// let prayer_times = fetch_jakim_prayer_times("SGR01", start, end).await?;
/// ```
pub async fn fetch_jakim_prayer_times(
    zone_code: &str,
    date_start: NaiveDate,
    date_end: NaiveDate,
) -> Result<Vec<PrayerTime>, Box<dyn std::error::Error>> {
    let base_url = "https://www.e-solat.gov.my/index.php";

    // Format dates as YYYY-MM-DD
    let date_start_str = date_start.format("%Y-%m-%d").to_string();
    let date_end_str = date_end.format("%Y-%m-%d").to_string();

    // Create form data
    let params = [
        ("datestart", date_start_str.as_str()),
        ("dateend", date_end_str.as_str()),
    ];

    // Create HTTP client
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    // Make POST request
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

    // Check if request was successful
    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()).into());
    }

    // Parse JSON response
    let jakim_response: JakimResponse = response.json().await?;

    // Validate response
    if jakim_response.status != "OK!" {
        return Err(format!("API returned status: {}", jakim_response.status).into());
    }

    Ok(jakim_response.prayer_time)
}

/// Fetches prayer times for a single day (convenience function)
pub async fn fetch_jakim_prayer_times_single_day(
    zone_code: &str,
    date: NaiveDate,
) -> Result<Option<PrayerTime>, Box<dyn std::error::Error>> {
    let prayer_times = fetch_jakim_prayer_times(zone_code, date, date).await?;
    Ok(prayer_times.into_iter().next())
}

/// Fetches prayer times for the next N days starting from today
pub async fn fetch_jakim_prayer_times_next_days(
    zone_code: &str,
    days: i64,
) -> Result<Vec<PrayerTime>, Box<dyn std::error::Error>> {
    let today = chrono::Local::now().date_naive();
    let end_date = today + chrono::Duration::days(days);
    fetch_jakim_prayer_times(zone_code, today, end_date).await
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

        // Check first entry has required fields
        let first = &prayer_times[0];
        assert!(!first.date.is_empty());
        assert!(!first.fajr.is_empty());
        assert!(!first.dhuhr.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_single_day() {
        let date = NaiveDate::from_ymd_opt(2025, 11, 10).unwrap();

        let result = fetch_jakim_prayer_times_single_day("WLY01", date).await;

        assert!(result.is_ok());
        let prayer_time = result.unwrap();
        assert!(prayer_time.is_some());
    }

    #[tokio::test]
    async fn test_fetch_next_7_days() {
        let result = fetch_jakim_prayer_times_next_days("JHR01", 7).await;

        assert!(result.is_ok());
        let prayer_times = result.unwrap();
        assert_eq!(prayer_times.len(), 7);
    }
}
