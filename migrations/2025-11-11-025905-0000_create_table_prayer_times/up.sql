-- Your SQL goes here
CREATE TABLE IF NOT EXISTS prayer_times (
    id BIGINT PRIMARY KEY,
    zone_code VARCHAR(10),
    date DATE,
    imsak TIME,
    fajr TIME,
    syuruk TIME,
    dhuhr TIME,
    asr TIME,
    maghrib TIME,
    isha TIME,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- - id (primary key)
-- - zone_code (string, e.g., "SGR01" for Johor Bahru)
-- - date (date)
-- - imsak (time)
-- - fajr (time)
-- - syuruk (time)
-- - dhuhr (time)
-- - asr (time)
-- - maghrib (time)
-- - isha (time)
-- - created_at (timestamp)
-- - updated_at (timestamp)