# Simplesolat API

> REST API for Malaysian prayer times using official Jakim data

**Live API:** https://api.simplesolat.apps.bancuh.net

A simple, reliable API that serves accurate prayer times for all 80 Malaysian prayer zones using data from JAKIM e-Solat.

---

## Features

- ✅ **Official Jakim Data** - Accurate prayer times from JAKIM e-Solat
- ✅ **All 80 Malaysian Zones** - Complete coverage (Johor, Sabah, Sarawak, etc.)
- ✅ **7 Prayer Times** - Imsak, Fajr, Syuruk, Dhuhr, Asr, Maghrib, Isha
- ✅ **Flexible Date Ranges** - Query any date range
- ✅ **Unix Timestamps** - Easy to use in mobile apps
- ✅ **Fast & Reliable** - Built with Rust + PostgreSQL
- ✅ **Auto-Sync** - Syncs prayer times from Jakim automatically

---

## Quick Start

### Get Prayer Times by zone

```bash
# Get prayer times for specific zone and date range
curl "https://api.simplesolat.apps.bancuh.net/prayer-times/by-zone/SGR01?from=2026-01-01&to=2026-01-31"
```

### Response

```json
{
  "data": [
    {
      "date": "2026-01-01",
      "zone": "SGR01",
      "imsak": 1735689480,
      "fajr": 1735689540,
      "syuruk": 1735693740,
      "dhuhr": 1735715340,
      "asr": 1735729740,
      "maghrib": 1735740540,
      "isha": 1735745040
    }
    // ... more days
  ]
}
```

---

## API Documentation

### Endpoint

```
GET /prayer-times/by-zone/:zone
```

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `zone` | string | Yes | Malaysian prayer zone code (e.g., `SGR01`, `WLY01`) |
| `from` | date | Yes | Start date (format: `YYYY-MM-DD`) |
| `to` | date | Yes | End date (format: `YYYY-MM-DD`) |

### Zone Codes

All 80 Malaysian prayer zones are supported. Examples:

- `WLY01` - W.P. Kuala Lumpur
- `SGR01` - Selangor (Gombak, Petaling, Sepang, etc.)
- `JHR01` - Johor (Pulau Aur, Pemanggil)
- `PNG01` - Pulau Pinang
- `SBH07` - Sabah (Kota Kinabalu, Ranau, etc.)
- `SWK08` - Sarawak (Kuching, Bau, Lundu)

[Full zone list](./data/zones.yaml)

### Response Format

```typescript
{
  data: Array<{
    date: string;        // ISO date (YYYY-MM-DD)
    zone: string;        // Zone code
    imsak: number;       // Unix timestamp (seconds)
    fajr: number;        // Unix timestamp (seconds)
    syuruk: number;      // Unix timestamp (seconds)
    dhuhr: number;       // Unix timestamp (seconds)
    asr: number;         // Unix timestamp (seconds)
    maghrib: number;     // Unix timestamp (seconds)
    isha: number;        // Unix timestamp (seconds)
  }>
}
```

All times are Unix timestamps (seconds since epoch) in Malaysia timezone (UTC+8).

### Examples

**Single Day:**
```bash
curl "https://api.simplesolat.apps.bancuh.net/prayer-times/by-zone/WLY01?from=2026-01-01&to=2026-01-01"
```

**Month:**
```bash
curl "https://api.simplesolat.apps.bancuh.net/prayer-times/by-zone/SGR01?from=2026-01-01&to=2026-01-31"
```

**Year Transition:**
```bash
curl "https://api.simplesolat.apps.bancuh.net/prayer-times/by-zone/JHR01?from=2025-12-15&to=2026-01-15"
```

**Next 90 Days:**
```bash
# From today
curl "https://api.simplesolat.apps.bancuh.net/prayer-times/by-zone/SGR01?from=2025-11-13&to=2026-02-11"
```

---

## Tech Stack

- **Rust** - Fast, safe, reliable
- **Axum** - Modern web framework
- **PostgreSQL** - Database
- **Diesel** - ORM
- **Docker** - Containerization
- **Jakim e-Solat API** - Data source

---

## Architecture

```
┌─────────────┐
│ Mobile App  │
└──────┬──────┘
       │ HTTPS
       ▼
┌─────────────┐      ┌──────────────┐
│ Axum API    │◄────►│ PostgreSQL   │
└──────┬──────┘      └──────────────┘
       │
       │ Daily Sync
       ▼
┌─────────────┐
│ Jakim API   │
└─────────────┘
```

### Data Flow

1. **Sync Job** - Runs daily to fetch prayer times from Jakim
2. **Database** - Stores prayer times for all zones
3. **API** - Serves data to mobile apps
4. **Cache** - Fast responses (in-memory caching planned)

---

## Development

### Prerequisites

- Rust 1.75+
- PostgreSQL 14+
- Docker (optional)

### Setup

1. **Clone the repository**
```bash
git clone https://github.com/ragibkl/simplesolat-api.git
cd simplesolat-api
```

2. **Start PostgreSQL**
```bash
docker-compose up -d
```

3. **Set environment variables**
```bash
cp .env.example .env
# Edit .env with your database credentials
```

4. **Run database migrations**
```bash
diesel migration run
```

5. **Sync prayer times data**
```bash
cargo run --bin sync
```

6. **Start the API server**
```bash
cargo run
```

The API will be available at `http://localhost:8080`

### Project Structure

```
simplesolat-api/
├── src/
│   ├── main.rs           # API server (Axum)
│   ├── lib.rs            # Shared library code
│   ├── db.rs             # Database queries
│   └── models.rs         # Data models
├── migrations/           # Diesel migrations
├── docker-compose.yml    # PostgreSQL setup
├── Cargo.toml            # Rust dependencies
├── zones.yaml            # All 80 Malaysian zones
└── README.md
```

### Running Tests

```bash
cargo test
```

### Building for Production

```bash
cargo build --release
./target/release/simplesolat-api
```

---

## Deployment

### Docker

```bash
# Build image
docker build -t simplesolat-api .

# Run container
docker run -p 8080:8080 \
  -e DATABASE_URL=postgresql://user:pass@host/db \
  simplesolat-api
```

### Environment Variables

```bash
DATABASE_URL=postgresql://user:pass@host:5432/prayer_times
PORT=8080
RUST_LOG=info
```

---

## Data Source

Prayer times are sourced from **JAKIM e-Solat** (Jabatan Kemajuan Islam Malaysia):
- Official API: https://www.e-solat.gov.my
- Method: Egyptian General Authority of Survey
- Timezone: Malaysia (UTC+8)
- Updates: Daily automatic sync

---

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---

## License

MIT License - see [LICENSE](LICENSE) file for details

---

## Related Projects

- **Simplesolat Mobile App** - React Native app for Android to display prayer times on app and on home screen widget (coming soon)

---

## Support

- **Issues:** [GitHub Issues](https://github.com/ragibkl/simplesolat-api/issues)
- **Email:** [your-email@example.com]
- **Website:** https://simplesolat.com (coming soon)

---

## Acknowledgments

- **JAKIM** - For providing official prayer times data
- **Rust Community** - For excellent tools and libraries
- **Malaysian Muslim Community** - For feedback and testing

---

Built with ❤️ for the Malaysian Muslim community

**"Prayer times that just work."**
