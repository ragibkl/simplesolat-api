# Simplesolat API

> Free REST API for official prayer times, served from the open [simplesolat-data](https://github.com/ragibkl/simplesolat-data) dataset

**Live API:** https://api.simplesolat.com

Find the prayer zone for a GPS location, then get its official timetable for a
day, a month or a year. Times are local (`"HH:MM"`, in the zone's timezone),
straight from each country's religious authority. No key, no sign-up. Use it,
or run your own copy (below): it's MIT licensed and has no database.

---

## Quick start

```bash
# 1. Which zone am I in?
curl -X POST https://api.simplesolat.com/v2/zones/by-location \
  -H "Content-Type: application/json" -d '{"lat": 3.139, "lng": 101.687}'

# 2. Today's prayer times for that zone
curl https://api.simplesolat.com/v2/zones/WLY01/prayer-times/day/2026-10-27
```

```json
{
  "zone": "WLY01",
  "timezone": "Asia/Kuala_Lumpur",
  "authority": "JAKIM",
  "data": {
    "date": "2026-10-27",
    "imsak": "05:38",
    "fajr": "05:48",
    "syuruk": "06:56",
    "dhuha": "07:21",
    "dhuhr": "13:00",
    "asr": "16:19",
    "maghrib": "18:59",
    "isha": "20:09"
  }
}
```

---

## Countries

| Country | Code | Authority | Zones |
|---------|------|-----------|-------|
| Malaysia | `MY` | [JAKIM e-Solat](https://www.e-solat.gov.my) | 60 |
| Singapore | `SG` | [MUIS](https://data.gov.sg) | 1 |
| Indonesia | `ID` | [Kemenag](https://equran.id) | 517 |
| Brunei | `BN` | [KHEU / MORA](https://www.mora.gov.bn) | 4 |
| Sri Lanka | `LK` | [ACJU](https://www.acju.lk) | 13 |
| United Arab Emirates | `AE` | [AWQAF](https://www.awqaf.gov.ae) | 60 |
| Turkey | `TR` | [Diyanet](https://namazvakitleri.diyanet.gov.tr) | 865 |
| Bosnia and Herzegovina | `BA` | [Islamic Community in BiH (Vaktija)](https://vaktija.ba) | 118 |
| Albania | `AL` | KMSH (Diyanet) | 20 |

`GET /v2/countries` always has the current list. Outside these countries there
is no official zone (`no_official_zone`); apps usually calculate times locally
instead (e.g. with [adhan](https://github.com/batoulapps/adhan-js)).

---

## API v2

One URL per kind of request, no optional query parameters. All responses are
JSON; lists come as `{"data": [...]}`, single items as `{"data": {...}}`.

### Find the zone for a location

`POST /v2/zones/by-location` with `{"lat": <-90..90>, "lng": <-180..180>}`

```json
{
  "data": {
    "zone": "WLY01",
    "country": "MY",
    "state": "W.P. Kuala Lumpur",
    "location": "Kuala Lumpur, Putrajaya",
    "timezone": "Asia/Kuala_Lumpur",
    "authority": "JAKIM",
    "district": "Kuala Lumpur"
  }
}
```

It's a POST so the location stays out of URLs, caches and logs; the API
doesn't log it. Lookups use the same district maps as the simplesolat app. The
first lookup after a restart loads the maps and takes a few seconds.

### Prayer times for a zone

| Endpoint | `data` |
|----------|--------|
| `GET /v2/zones/{zone}/prayer-times/day/{YYYY-MM-DD}` | one day (object) |
| `GET /v2/zones/{zone}/prayer-times/month/{YYYY-MM}` | every day of the month (list) |
| `GET /v2/zones/{zone}/prayer-times/year/{YYYY}` | every day of the year (list) |

Each day has `date`, `imsak`, `fajr`, `syuruk`, `dhuhr`, `asr`, `maghrib` and
`isha`, plus `dhuha` where the authority publishes it (Malaysia, Brunei,
Indonesia). Times are `"HH:MM"`, 24-hour, local to the zone's `timezone`. Days
the authority hasn't published yet are left out; if none are published the
response is `no_prayer_times` (404).

### Zones and countries

| Endpoint | Returns |
|----------|---------|
| `GET /v2/zones` | every zone |
| `GET /v2/zones/{zone}` | one zone |
| `GET /v2/countries` | every country (`code`, `name`, `authority`) |
| `GET /v2/countries/{country}` | one country |
| `GET /v2/countries/{country}/zones` | that country's zones |

A zone has `zone`, `country`, `state`, `location`, `timezone` and `authority`.
Zone codes are unique across countries (`WLY01`, `SGP01`, `DKI02`, `BRN01`...).

### Errors

```json
{"error": {"code": "zone_not_found", "message": "no zone 'XYZ01'; see /v2/zones"}}
```

| Code | Status |
|------|--------|
| `invalid_date`, `invalid_month`, `invalid_year`, `invalid_location` | 400 |
| `zone_not_found`, `country_not_found`, `no_official_zone`, `no_prayer_times` | 404 |
| `data_source_unavailable` | 502: simplesolat-data couldn't be reached; try again |

### Caching

Everything is fetched from the simplesolat-data CDN on demand and kept in
memory; there are no background jobs.

- **Zones and countries**: cached for `ZONES_CACHE_TTL`.
- **District maps** (for `by-location`): loaded together on the first lookup,
  cached for `ZONES_CACHE_TTL`.
- **Prayer times**: one cache entry per month file. Published months are cached
  for `PRAYER_TIMES_CACHE_TTL`; months not published yet for the shorter
  `PRAYER_TIMES_MISSING_CACHE_TTL`, so new months show up sooner.
- **CDN failures** are never cached. If a refetch fails, the last good copy is
  served (zones and maps indefinitely, months for up to 30 days).

---

## API v1 (legacy)

Kept for simplesolat app 1.0.x installs. New code should use v2.

| Endpoint | Returns |
|----------|---------|
| `GET /prayer-times/by-zone/{zone}?from=YYYY-MM-DD&to=YYYY-MM-DD` | times as Unix timestamps, up to 750 days |
| `GET /zones?country=MY` | zones |
| `GET /countries` | countries with GeoJSON and mapping URLs |
| `GET /health` | `{"service": "simplesolat-api", "status": "ok"}` (doesn't touch the CDN) |

---

## Run your own copy

The API has no database: it reads the public simplesolat-data CDN (or your own
copy of it, via `DATA_BASE_URL`). It uses roughly 110 MB of memory with the maps loaded.

### Docker Compose

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
- [simplesolat](https://github.com/ragibkl/simplesolat) — the app (Android and iPhone), [simplesolat.com](https://simplesolat.com)

Found a wrong time or zone? Please open an issue or pull request on
[simplesolat-data](https://github.com/ragibkl/simplesolat-data): the API only
serves what's there.

---

## License

MIT
