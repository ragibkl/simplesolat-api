# Simplesolat API

> REST API for prayer times — Malaysia, Singapore, Indonesia, Brunei, and Sri Lanka

**Live API:** https://api.simplesolat.com

---

## Features

- **594 zones** across 5 countries (MY, SG, ID, BN, LK)
- **7 prayer times** — Imsak, Fajr, Syuruk, Dhuhr, Asr, Maghrib, Isha
- **Unix timestamps** — timezone-aware (UTC+5:30 to UTC+9)
- **Auto-sync** — syncs from [simplesolat-data](https://github.com/ragibkl/simplesolat-data) repo
- Built with **Rust + Axum + PostgreSQL**

## Data Source

Prayer times are sourced from [simplesolat-data](https://github.com/ragibkl/simplesolat-data), a centralized data repo that aggregates official prayer times from:

| Country | Authority | Zones |
|---------|-----------|-------|
| Malaysia | [JAKIM e-Solat](https://www.e-solat.gov.my) | 60 |
| Singapore | [MUIS](https://data.gov.sg) | 1 |
| Indonesia | [Kemenag](https://equran.id) | 517 |
| Brunei | [KHEU / MORA](https://www.mora.gov.bn) | 4 |
| Sri Lanka | [ACJU](https://www.acju.lk) | 13 |

---

## Quick Start

```bash
# Get prayer times for a zone
curl "https://api.simplesolat.com/prayer-times/by-zone/SGR01?from=2026-01-01&to=2026-01-31"

# List all zones
curl "https://api.simplesolat.com/zones"

# List zones for a specific country
curl "https://api.simplesolat.com/zones?country=LK"

# List supported countries
curl "https://api.simplesolat.com/countries"

# Health check
curl "https://api.simplesolat.com/health"
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
  ]
}
```

All times are Unix timestamps (seconds) in the zone's local timezone.

---

## API Endpoints

### `GET /prayer-times/by-zone/:zone`

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `zone` | path | Yes | Zone code (e.g. `SGR01`, `SGP01`, `ACH01`, `BRN01`, `LK01`) |
| `from` | query | Yes | Start date (`YYYY-MM-DD`) |
| `to` | query | Yes | End date (`YYYY-MM-DD`) |

### `GET /zones`

Returns all zones with `zone`, `country`, `state`, `location`, and `timezone` fields.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `country` | query | No | Filter by country code (e.g. `MY`, `LK`) |

### `GET /countries`

Returns supported countries with geojson and mapping file URLs (for mobile zone resolution).

### `GET /health`

Returns `{"service": "simplesolat-api", "status": "ok", "db": "connected"}`. Returns HTTP 503 if the database is unavailable.

### Zone Codes

- **Malaysia** — 3-letter state + 2-digit: `SGR01`, `WLY01`, `JHR02`
- **Singapore** — `SGP01`
- **Indonesia** — 3-letter province + 2-digit: `ACH01` (Aceh), `JTM38` (Jawa Timur), `DKI02` (Jakarta)
- **Brunei** — `BRN01` (Brunei-Muara), `BRN02` (Tutong), `BRN03` (Belait), `BRN04` (Temburong)
- **Sri Lanka** — `LK01`-`LK13` (ACJU official zones, e.g. LK01 = Colombo/Gampaha/Kalutara)

Zone definitions are managed in [simplesolat-data](https://github.com/ragibkl/simplesolat-data).

---

## Self-Hosting with Docker Compose

```yaml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: user
      POSTGRES_PASSWORD: password
      POSTGRES_DB: simplesolat_db
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U user -d simplesolat_db"]
      interval: 5s
      timeout: 5s
      retries: 5

  simplesolat-api:
    image: ghcr.io/ragibkl/simplesolat-api:latest
    environment:
      DATABASE_URL: postgres://user:password@postgres/simplesolat_db
    ports:
      - 3000:3000
    depends_on:
      postgres:
        condition: service_healthy

  simplesolat-sync:
    image: ghcr.io/ragibkl/simplesolat-api:latest
    command: ["simplesolat-api", "sync", "--loop", "6h"]
    environment:
      DATABASE_URL: postgres://user:password@postgres/simplesolat_db
    depends_on:
      postgres:
        condition: service_healthy

volumes:
  pgdata:
```

> **Note:** The first sync fetches all prayer times from GitHub Pages (~8 minutes for all 594 zones). Subsequent syncs are fast — only new data is fetched.

### CLI Usage

```bash
# Start API server (default)
simplesolat-api
simplesolat-api serve

# Sync all countries (one-shot)
simplesolat-api sync

# Sync a specific country
simplesolat-api sync --country MY

# Sync in loop mode (for docker-compose)
simplesolat-api sync --loop 6h
```

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | Yes | — | PostgreSQL connection string |
| `PORT` | No | `3000` | API server port |
| `RUST_LOG` | No | `info` | Log level |

---

## Development

```bash
# Start postgres
docker-compose up -d postgres

# Copy env
cp sample.env .env
# Edit .env with your values

# Run sync
cargo run -- sync

# Start API
cargo run
```

---

## Related Projects

- [simplesolat-data](https://github.com/ragibkl/simplesolat-data) — Centralized prayer times data repo (zones, mappings, GeoJSON)
- [simplesolat](https://github.com/ragibkl/simplesolat) — Android app on Google Play Store

---

## License

MIT
