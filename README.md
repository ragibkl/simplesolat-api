# Simplesolat API

> REST API for prayer times — Malaysia, Singapore, Indonesia, and Brunei

**Live API:** https://api.simplesolat.com

---

## Features

- **594 zones** across 5 countries (MY, SG, ID, BN, LK)
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
| Sri Lanka | [ACJU](https://www.acju.lk) (static data) | 13 zones |

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
- **Sri Lanka** — `LK01`-`LK13` (ACJU official zones, e.g. LK01 = Colombo/Gampaha/Kalutara)

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
simplesolat-api sync acju

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

## Data Source Coverage

### Integrated (official data, served via API)

| Country | Authority | Source | Zones | Sync |
|---------|-----------|--------|-------|------|
| Malaysia | JAKIM | [e-solat.gov.my](https://www.e-solat.gov.my) | 60 | Daily 3 AM |
| Singapore | MUIS | [data.gov.sg](https://data.gov.sg) CKAN API | 1 | Daily 4 AM |
| Indonesia | Kemenag | [equran.id](https://equran.id) (wrapper) | 517 | Weekly Mon 5 AM |
| Brunei | KHEU / MORA | [mora.gov.bn](https://www.mora.gov.bn) SharePoint | 4 | Daily 5 AM |
| Sri Lanka | ACJU | Static JSON from [thani-sh/prayer-time-lk](https://github.com/thani-sh/prayer-time-lk) (MIT) | 13 | Daily 6 AM |

### Official sources to investigate (potential future integration)

| Country | Authority | Source | Notes |
|---------|-----------|--------|-------|
| Bangladesh | Islamic Foundation | [islamicfoundation.gov.bd](https://islamicfoundation.gov.bd) | Publishes district-level schedules, check if scrapeable |
| Turkey | Diyanet | [awqatsalah.diyanet.gov.tr](https://awqatsalah.diyanet.gov.tr) | Has REST API, heavily rate-limited. Currently calculated on mobile. |
| UAE | IACAD | [iacad.gov.ae](https://www.iacad.gov.ae/en/open-data/prayer-time-open-data) | Open data portal. Currently calculated on mobile. |
| Morocco | Habous Ministry | [habous.gov.ma](https://habous.gov.ma) | Unofficial GitHub scraper exists |

### Worldwide coverage (calculation on mobile via adhan-js)

For countries without official API sources, the [mobile app](https://github.com/ragibkl/simplesolat) calculates prayer times client-side using [adhan-js](https://github.com/batoulapps/adhan-js) with region-appropriate methods.

**High confidence** — well-defined official method, supported by adhan-js:

| Country | Method | Fajr / Isha | Notes |
|---------|--------|-------------|-------|
| Saudi Arabia | Umm Al-Qura University | 18.5° / 90min | Official govt standard, used by Haramain |
| Egypt | Egyptian General Authority of Survey | 19.5° / 17.5° | Widely used across Africa |
| Qatar | Qatar | 18° / 90min | |
| Kuwait | Kuwait | 18° / 17.5° | |
| Iran | Geophysics Institute Tehran | 17.7° / 14° | |
| US / Canada | ISNA | 15° / 15° | |

**Moderate confidence** — named method, not verified against local authority:

| Country | Method | Fajr / Isha | Notes |
|---------|--------|-------------|-------|
| Turkey | Diyanet | 18° / 17° | Pending official API integration |
| UAE | Dubai | 18.2° / 18.2° | Pending official API integration |
| Jordan | Jordan | 18° / 18° | |
| Algeria | Algerian Ministry | 18° / 17° | |
| Tunisia | Tunisia | 18° / 18° | |
| France | UOIF | 12° / 12° | |
| Pakistan | Karachi | 18° / 18° | Multiple methods used regionally |
| Russia | Russia | 16° / 15° | Regional variation |

**Low confidence** — no documented official method, best guess:

| Country | Best guess | Notes |
|---------|-----------|-------|
| Thailand | MWL or JAKIM-like (20°/18°) | CICOT is official body but method undocumented |
| Philippines | MWL (18°/17°) | NCMF announces Ramadan dates but no prayer times method |
| India | Karachi or MWL | No single authority, varies by region |
| Bangladesh | Karachi (assumed) | Islamic Foundation publishes times but angles undocumented |
| Oman, Bahrain, Yemen, Iraq | MWL (assumed) | Gulf states, no documented methods found |
| Libya, Sudan, Somalia | Egyptian or MWL (assumed) | No documented methods found |
| Maldives | MWL (assumed) | No documented method |
| All other countries | Muslim World League | Default fallback |

---

## Related Projects

- [simplesolat](https://github.com/ragibkl/simplesolat) — Android app on Google Play Store

---

## License

MIT
