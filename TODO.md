## TODO

### Should fix soon
1. **Health check doesn't check DB** — `src/routes/health.rs` returns 200 even if postgres is down. K8s liveness probe won't catch DB issues.

### Good to fix
2. **MUIS always fetches 155KB** — `src/service/prayer_times.rs` could check last date and skip fetch.
3. **No retry on API errors** — all sync functions log and skip on error. Transient failures mean lost data until next cron.
4. **`.unwrap()` on DB queries** — `src/models/zones.rs`, `src/models/prayer_times.rs`. DB hiccup crashes the process.

### Nice to have
5. **No caching / cache headers** — prayer times are immutable, could add `Cache-Control`.
6. **No rate limiting** — public API, could be abused.
