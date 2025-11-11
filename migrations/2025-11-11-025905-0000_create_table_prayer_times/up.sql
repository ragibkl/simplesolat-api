-- Your SQL goes here
CREATE TABLE IF NOT EXISTS prayer_times (
    id BIGINT PRIMARY KEY,
    zone_code VARCHAR(10) NOT NULL,
    date DATE NOT NULL,
    imsak TIME NOT NULL,
    fajr TIME NOT NULL,
    syuruk TIME NOT NULL,
    dhuhr TIME NOT NULL,
    asr TIME NOT NULL,
    maghrib TIME NOT NULL,
    isha TIME NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
