use chrono::{Days, NaiveDate, NaiveTime, Timelike};
use serde::{self, Deserialize, Deserializer};

const BRUNEI_ZONES: [(&str, i64); 4] = [
    ("BRN01", 0), // Brunei-Muara
    ("BRN02", 1), // Tutong (+1 min)
    ("BRN03", 3), // Belait (+3 min)
    ("BRN04", 0), // Temburong
];

/// Parse KHEU dot-separated time string to NaiveTime.
/// Handles upstream typos:
///   - "112.28" (extra digit) -> "12.28" -> 12:28
///   - "741" (missing dot) -> "7:41"
fn parse_time(time_str: &str, is_pm: bool) -> Result<NaiveTime, String> {
    // Insert missing dot: "741" -> "7.41", "1234" -> "12.34"
    let with_dot = if !time_str.contains('.') && time_str.len() >= 3 {
        let (h, m) = time_str.split_at(time_str.len() - 2);
        tracing::warn!("[kheu] inserting missing dot: '{}' -> '{}.{}'", time_str, h, m);
        format!("{}.{}", h, m)
    } else {
        time_str.to_string()
    };

    let normalized = with_dot.replace('.', ":");
    let t = NaiveTime::parse_from_str(&normalized, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(&normalized, "%-H:%M"))
        .or_else(|_| {
            // Retry with first char trimmed for typos like "112:28" -> "12:28"
            let trimmed = &normalized[1..];
            tracing::warn!("[kheu] trimming extra digit: '{}' -> '{}'", time_str, trimmed);
            NaiveTime::parse_from_str(trimmed, "%H:%M")
                .or_else(|_| NaiveTime::parse_from_str(trimmed, "%-H:%M"))
        })
        .map_err(|e| format!("invalid time '{}': {}", time_str, e))?;
    if is_pm && t.hour() < 12 {
        Ok(t + chrono::Duration::hours(12))
    } else {
        Ok(t)
    }
}

fn apply_offset(time: NaiveTime, offset_minutes: i64) -> NaiveTime {
    if offset_minutes == 0 {
        return time;
    }
    time + chrono::Duration::minutes(offset_minutes)
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

/// KHEU date is ISO with UTC offset: "2026-03-01T16:00:00Z" = next day in BN time (UTC+8).
fn deserialize_kheu_date<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let date = NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d")
        .map_err(serde::de::Error::custom)?;
    date.checked_add_days(Days::new(1))
        .ok_or_else(|| serde::de::Error::custom("date overflow"))
}

/// Parsed KHEU prayer times, matching upstream field names with offset applied.
pub struct KheuPrayerTime {
    pub zone_code: String,
    pub date: NaiveDate,
    pub imsak: NaiveTime,
    pub suboh: NaiveTime,
    pub syuruk: NaiveTime,
    pub zohor: NaiveTime,
    pub asar: NaiveTime,
    pub maghrib: NaiveTime,
    pub isyak: NaiveTime,
}

#[derive(Debug, Deserialize)]
struct RawKheuRecord {
    #[serde(rename = "Date", deserialize_with = "deserialize_kheu_date")]
    date: NaiveDate,
    #[serde(rename = "Imsak", deserialize_with = "deserialize_am")]
    imsak: NaiveTime,
    #[serde(rename = "Suboh", deserialize_with = "deserialize_am")]
    suboh: NaiveTime,
    #[serde(rename = "Syuruk", deserialize_with = "deserialize_am")]
    syuruk: NaiveTime,
    #[serde(rename = "Zohor", deserialize_with = "deserialize_pm")]
    zohor: NaiveTime,
    #[serde(rename = "Asar", deserialize_with = "deserialize_pm")]
    asar: NaiveTime,
    #[serde(rename = "Maghrib", deserialize_with = "deserialize_pm")]
    maghrib: NaiveTime,
    #[serde(rename = "Isyak", deserialize_with = "deserialize_pm")]
    isyak: NaiveTime,
}

#[derive(Debug, Deserialize)]
struct SharePointResponse {
    value: Vec<RawKheuRecord>,
    #[serde(rename = "odata.nextLink")]
    next_link: Option<String>,
}

impl RawKheuRecord {
    fn to_prayer_time(&self, zone_code: &str, offset: i64) -> KheuPrayerTime {
        KheuPrayerTime {
            zone_code: zone_code.to_string(),
            date: self.date,
            imsak: apply_offset(self.imsak, offset),
            suboh: apply_offset(self.suboh, offset),
            syuruk: apply_offset(self.syuruk, offset),
            zohor: apply_offset(self.zohor, offset),
            asar: apply_offset(self.asar, offset),
            maghrib: apply_offset(self.maghrib, offset),
            isyak: apply_offset(self.isyak, offset),
        }
    }
}

/// Fetches prayer times from KHEU via SharePoint REST API for a date range.
/// Handles pagination and returns parsed prayer times for all 4 Brunei zones with offsets applied.
pub async fn fetch_kheu_prayer_times(
    date_start: NaiveDate,
    date_end: NaiveDate,
) -> Result<Vec<KheuPrayerTime>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let start_date = format!("{}T00:00:00", date_start);
    let end_date = format!("{}T00:00:00", date_end);

    let base_url = format!(
        "https://www.mora.gov.bn/_api/web/lists/getbytitle('Waktu%20Sembahyang')/items\
        ?$select=Date,Imsak,Suboh,Syuruk,Zohor,Asar,Maghrib,Isyak\
        &$filter=Date ge datetime'{}' and Date lt datetime'{}'\
        &$orderby=Date asc\
        &$top=1000",
        start_date, end_date
    );

    let mut raw_records = Vec::new();
    let mut url = base_url;

    loop {
        let response = client
            .get(&url)
            .header("Accept", "application/json;odata=nometadata")
            .header("User-Agent", "Simplesolat/1.0")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("KHEU API returned status: {}", response.status()).into());
        }

        let sp_response: SharePointResponse = response.json().await?;
        raw_records.extend(sp_response.value);

        match sp_response.next_link {
            Some(next) => url = next,
            None => break,
        }
    }

    let mut prayer_times = Vec::new();
    for (zone_code, offset) in BRUNEI_ZONES {
        for r in &raw_records {
            prayer_times.push(r.to_prayer_time(zone_code, offset));
        }
    }

    Ok(prayer_times)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_imsak_am() {
        let t = parse_time("5.04", false).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(5, 4, 0).unwrap());
    }

    #[test]
    fn test_parse_time_asar_pm() {
        let t = parse_time("3.50", true).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(15, 50, 0).unwrap());
    }

    #[test]
    fn test_parse_time_typo_missing_dot() {
        // "741" is a real upstream typo for "7.41"
        let t = parse_time("741", true).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(19, 41, 0).unwrap());
    }

    #[test]
    fn test_parse_time_typo_extra_digit() {
        // "112.28" is a real upstream typo for "12.28"
        let t = parse_time("112.28", true).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(12, 28, 0).unwrap());
    }

    #[test]
    fn test_parse_time_zohor_pm_noon() {
        let t = parse_time("12.25", true).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(12, 25, 0).unwrap());
    }

    #[test]
    fn test_parse_time_maghrib_pm() {
        let t = parse_time("6.33", true).unwrap();
        assert_eq!(t, NaiveTime::from_hms_opt(18, 33, 0).unwrap());
    }

    #[test]
    fn test_zone_offsets() {
        assert_eq!(BRUNEI_ZONES[0], ("BRN01", 0));
        assert_eq!(BRUNEI_ZONES[1], ("BRN02", 1));
        assert_eq!(BRUNEI_ZONES[2], ("BRN03", 3));
        assert_eq!(BRUNEI_ZONES[3], ("BRN04", 0));
    }

    #[test]
    fn test_apply_offset_zero() {
        let t = NaiveTime::from_hms_opt(5, 4, 0).unwrap();
        assert_eq!(apply_offset(t, 0), t);
    }

    #[test]
    fn test_apply_offset_one_minute() {
        let t = NaiveTime::from_hms_opt(5, 4, 0).unwrap();
        assert_eq!(apply_offset(t, 1), NaiveTime::from_hms_opt(5, 5, 0).unwrap());
    }

    #[test]
    fn test_apply_offset_three_minutes() {
        let t = NaiveTime::from_hms_opt(18, 33, 0).unwrap();
        assert_eq!(apply_offset(t, 3), NaiveTime::from_hms_opt(18, 36, 0).unwrap());
    }
}
