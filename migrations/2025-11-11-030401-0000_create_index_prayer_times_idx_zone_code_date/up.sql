-- Your SQL goes here
CREATE UNIQUE INDEX IF NOT EXISTS idx_prayer_times_zone_code_date ON prayer_times (zone_code, date);
