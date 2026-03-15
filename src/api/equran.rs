use serde::Deserialize;

/// Represents a single day's prayer times from EQuran.id
#[derive(Debug, Deserialize)]
pub struct EquranSchedule {
    pub tanggal_lengkap: String, // "YYYY-MM-DD"
    pub imsak: String,           // "HH:MM" 24-hour
    pub subuh: String,
    pub terbit: String,
    pub dzuhur: String,
    pub ashar: String,
    pub maghrib: String,
    pub isya: String,
}

#[derive(Debug, Deserialize)]
struct EquranData {
    jadwal: Vec<EquranSchedule>,
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

/// Fetches monthly prayer times from EQuran.id for a given province/city.
pub async fn fetch_equran_prayer_times(
    provinsi: &str,
    kabkota: &str,
    bulan: i32,
    tahun: i32,
) -> Result<Vec<EquranSchedule>, Box<dyn std::error::Error>> {
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

    Ok(equran.data.jadwal)
}
