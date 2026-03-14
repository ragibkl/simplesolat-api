use serde::Deserialize;

/// Represents a single day's prayer times from MUIS (data.gov.sg)
#[derive(Debug, Deserialize)]
pub struct MuisRecord {
    #[serde(rename = "Date")]
    pub date: String, // "YYYY-MM-DD"
    #[serde(rename = "Subuh")]
    pub subuh: String, // "HH:MM"
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
struct CkanResult {
    records: Vec<MuisRecord>,
}

#[derive(Debug, Deserialize)]
struct CkanResponse {
    result: CkanResult,
}

/// Fetches all prayer times from MUIS via data.gov.sg CKAN API
///
/// Returns the consolidated dataset (2024-2026, ~1100 records) in a single request.
pub async fn fetch_muis_prayer_times() -> Result<Vec<MuisRecord>, Box<dyn std::error::Error>> {
    let url = "https://data.gov.sg/api/action/datastore_search";

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client
        .get(url)
        .header("User-Agent", "Simplesolat/1.0")
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
    Ok(ckan.result.records)
}
