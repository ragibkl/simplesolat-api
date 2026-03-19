use chrono::{NaiveDate, NaiveTime};
use serde::Deserialize;

/// Parsed EQuran.id prayer times, deserialized directly from upstream JSON.
#[derive(Debug, Deserialize)]
pub struct EquranPrayerTime {
    #[serde(skip)]
    pub zone_code: String,
    pub tanggal_lengkap: NaiveDate,
    pub imsak: NaiveTime,
    pub subuh: NaiveTime,
    pub terbit: NaiveTime,
    pub dzuhur: NaiveTime,
    pub ashar: NaiveTime,
    pub maghrib: NaiveTime,
    pub isya: NaiveTime,
}

#[derive(Debug, Deserialize)]
struct EquranData {
    jadwal: Vec<EquranPrayerTime>,
}

#[derive(Debug, Deserialize)]
struct EquranResponse {
    code: i32,
    data: EquranData,
}

#[derive(serde::Serialize)]
struct EquranRequest {
    provinsi: String,
    kabkota: String,
    bulan: i32,
    tahun: i32,
}

pub async fn fetch_equran_prayer_times(
    zone_code: &str,
    provinsi: &str,
    kabkota: &str,
    bulan: i32,
    tahun: i32,
) -> Result<Vec<EquranPrayerTime>, Box<dyn std::error::Error>> {
    let url = "https://equran.id/api/v2/shalat";

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client
        .post(url)
        .header("User-Agent", "Simplesolat/1.0")
        .json(&EquranRequest {
            provinsi: provinsi.to_string(),
            kabkota: kabkota.to_string(),
            bulan,
            tahun,
        })
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("EQuran.id API returned status: {}", response.status()).into());
    }

    let equran: EquranResponse = response.json().await?;
    if equran.code != 200 {
        return Err(format!("EQuran.id returned code: {}", equran.code).into());
    }

    let mut prayer_times = equran.data.jadwal;
    for pt in &mut prayer_times {
        pt.zone_code = zone_code.to_string();
    }
    Ok(prayer_times)
}
