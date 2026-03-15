use serde::Deserialize;

/// Represents a single day's prayer times from KHEU (Brunei Ministry of Religious Affairs)
#[derive(Debug, Deserialize)]
pub struct KheuRecord {
    #[serde(rename = "Date")]
    pub date: String, // ISO datetime "2026-03-01T16:00:00Z"
    #[serde(rename = "Imsak")]
    pub imsak: String, // "5.04" (dot-separated, 12-hour)
    #[serde(rename = "Suboh")]
    pub suboh: String,
    #[serde(rename = "Syuruk")]
    pub syuruk: String,
    #[serde(rename = "Zohor")]
    pub zohor: String,
    #[serde(rename = "Asar")]
    pub asar: String,
    #[serde(rename = "Maghrib")]
    pub maghrib: String,
    #[serde(rename = "Isyak")]
    pub isyak: String,
}

#[derive(Debug, Deserialize)]
struct SharePointResponse {
    value: Vec<KheuRecord>,
    #[serde(rename = "odata.nextLink")]
    next_link: Option<String>,
}

/// Fetches prayer times from KHEU via SharePoint REST API for a given year and month.
/// Returns all records for the month, handling pagination.
pub async fn fetch_kheu_prayer_times(
    year: i32,
    month: u32,
) -> Result<Vec<KheuRecord>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let (next_month_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };

    let start_date = format!("{}-{:02}-01T00:00:00", year, month);
    let end_date = format!("{}-{:02}-01T00:00:00", next_month_year, next_month);

    let base_url = format!(
        "https://www.mora.gov.bn/_api/web/lists/getbytitle('Waktu%20Sembahyang')/items\
        ?$select=Date,Imsak,Suboh,Syuruk,Zohor,Asar,Maghrib,Isyak\
        &$filter=Date ge datetime'{}' and Date lt datetime'{}'\
        &$orderby=Date asc\
        &$top=100",
        start_date, end_date
    );

    let mut all_records = Vec::new();
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
        all_records.extend(sp_response.value);

        match sp_response.next_link {
            Some(next) => url = next,
            None => break,
        }
    }

    Ok(all_records)
}
