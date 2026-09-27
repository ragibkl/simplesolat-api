# Simplesolat API

> REST API for prayer times, served from the [simplesolat-data](https://github.com/ragibkl/simplesolat-data) CDN

**Live API:** https://api.simplesolat.com

The simplesolat app reads the CDN directly since 1.1.0. This API stays up for
1.0.x installs, which call `/prayer-times/by-zone/:zone`. It has no database:
it fetches the CDN's static files on demand and caches them in memory. The
Postgres-backed version with sync workers is tagged `v1-postgres`.

---

## Features

- **Every zone in simplesolat-data** (9 countries, 1,658 zones as of 2026-09)
- **7 prayer times** — Imsak, Fajr, Syuruk, Dhuhr, Asr, Maghrib, Isha
- **Unix timestamps** — timezone-aware, per zone
- **Stateless** — reads the CDN on demand, no database or sync jobs
- Built with **Rust + Axum**

## Data Source

Prayer times are sourced from [simplesolat-data](https://github.com/ragibkl/simplesolat-data), a centralized data repo that aggregates official prayer times, including from:

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

Returns `{"service": "simplesolat-api", "status": "ok"}`. Liveness only: it doesn't touch the CDN, so probes never cause fetches.

### Caching

Everything is fetched on demand by the request that needs it; there are no background fetches.

- **Zones and countries** (`countries.yaml`, `zones/*.yaml`): cached for `ZONES_CACHE_TTL`.
- **Prayer times**: one cache entry per month file. Published months are cached for `PRAYER_TIMES_CACHE_TTL`; months not published yet (404) for the shorter `PRAYER_TIMES_MISSING_CACHE_TTL`, so new months show up sooner.
- **CDN failures** are never cached. If a refetch fails, the last good copy is served (zones indefinitely, months for up to 30 days); only data that was never fetched returns a 502.
- Concurrent requests for the same file share one fetch, and a request fetches all the months it needs in parallel.

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
  simplesolat-api:
    image: ghcr.io/ragibkl/simplesolat-api:latest
    ports:
      - 3000:3000
```

### CLI Usage

```bash
# Start API server (default)
simplesolat-api
simplesolat-api serve
```

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATA_BASE_URL` | No | `https://simplesolat-data.netlify.app` | simplesolat-data CDN |
| `ZONES_CACHE_TTL` | No | `1d` | Cache time for zones and countries |
| `PRAYER_TIMES_CACHE_TTL` | No | `1d` | Cache time for a published month |
| `PRAYER_TIMES_MISSING_CACHE_TTL` | No | `1h` | Cache time for a month not published yet |
| `PORT` | No | `3000` | API server port |
| `RUST_LOG` | No | `info` | Log level |

Durations take `s`, `m`, `h` or `d` (e.g. `30m`).

---

## Development

```bash
# Start API
cargo run

# Unit tests (the data_repo ones hit the live CDN)
cargo test --lib

# E2E tests, against the API running on localhost:3000
cargo test --test e2e
```

---

## Related Projects

- [simplesolat-data](https://github.com/ragibkl/simplesolat-data) — Centralized prayer times data repo (zones, mappings, GeoJSON)
- [simplesolat](https://github.com/ragibkl/simplesolat) — Android app on Google Play Store

---

## License

MIT
