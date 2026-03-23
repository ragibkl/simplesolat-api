# Simplesolat API

> REST API for prayer times — Malaysia, Singapore, Indonesia, and Brunei

**Live API:** https://api.simplesolat.com

---

## Features

- **582 zones** across 4 countries (MY, SG, ID, BN)
- **7 prayer times** — Imsak, Fajr, Syuruk, Dhuhr, Asr, Maghrib, Isha
- **Unix timestamps** — timezone-aware (UTC+7/+8/+9)
- **Auto-sync** — daily/weekly sync from official sources
- Built with **Rust + Axum + PostgreSQL**

## Data Sources

| Country | Source | Zones |
|---------|--------|-------|
| Malaysia | [JAKIM e-Solat](https://www.e-solat.gov.my) | 60 zones |
| Singapore | [MUIS via data.gov.sg](https://data.gov.sg) | 1 zone |
| Indonesia | [EQuran.id](https://equran.id) (wraps Kemenag) | 517 zones |
| Brunei | [KHEU / MORA](https://www.mora.gov.bn) | 4 zones (with minute offsets) |

---

## Quick Start

```bash
# Get prayer times for a zone
curl "https://api.simplesolat.com/prayer-times/by-zone/SGR01?from=2026-01-01&to=2026-01-31"

# List all zones
curl "https://api.simplesolat.com/zones"

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
| `zone` | path | Yes | Zone code (e.g. `SGR01`, `SGP01`, `ACH01`, `BRN01`) |
| `from` | query | Yes | Start date (`YYYY-MM-DD`) |
| `to` | query | Yes | End date (`YYYY-MM-DD`) |

### `GET /zones`

Returns all zones with `zone`, `country`, `state`, and `location` fields.

### `GET /health`

Returns `{"service": "simplesolat-api", "status": "ok", "db": "connected"}`. Returns HTTP 503 if the database is unavailable.

### Zone Codes

- **Malaysia** — 3-letter state + 2-digit: `SGR01`, `WLY01`, `JHR02`
- **Singapore** — `SGP01`
- **Indonesia** — 3-letter province + 2-digit: `ACH01` (Aceh), `JTM38` (Jawa Timur), `DKI02` (Jakarta)
- **Brunei** — `BRN01` (Brunei-Muara), `BRN02` (Tutong), `BRN03` (Belait), `BRN04` (Temburong)

Full zone list: [data/zones.yaml](./data/zones.yaml)

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
      # MUIS_API_KEY: <optional, get one at https://data.gov.sg for Singapore data>
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
      # MUIS_API_KEY: <optional, get one at https://data.gov.sg for Singapore data>
    depends_on:
      postgres:
        condition: service_healthy

volumes:
  pgdata:
```

> **Note:** The first sync fetches data from all 4 upstream sources. JAKIM, MUIS, and KHEU complete in a few minutes, but EQuran (517 Indonesian zones) takes ~20 minutes. You can sync individual sources if needed: `simplesolat-api sync jakim`.

### CLI Usage

```bash
# Start API server (default)
simplesolat-api
simplesolat-api serve

# Sync all sources (one-shot)
simplesolat-api sync

# Sync specific source
simplesolat-api sync jakim
simplesolat-api sync muis
simplesolat-api sync equran
simplesolat-api sync kheu

# Sync in loop mode (for docker-compose)
simplesolat-api sync --loop 6h
simplesolat-api sync jakim --loop 1d
```

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | Yes | — | PostgreSQL connection string |
| `PORT` | No | `3000` | API server port |
| `RUST_LOG` | No | `info` | Log level |
| `MUIS_API_KEY` | No | — | data.gov.sg API key (for higher rate limits) |

---

## Development

```bash
# Start postgres
docker-compose up -d postgres

# Copy env
cp sample.env .env
# Edit .env with your values

# Run sync
cargo run -- sync jakim

# Start API
cargo run
```

---

## Related Projects

- [simplesolat](https://github.com/ragibkl/simplesolat) — Android app on Google Play Store

---

## License

MIT
