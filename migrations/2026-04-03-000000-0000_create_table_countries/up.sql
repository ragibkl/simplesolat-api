CREATE TABLE countries (
    code VARCHAR(2) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    source VARCHAR(100) NOT NULL,
    geojson TEXT NOT NULL,
    mapping TEXT NOT NULL,
    shape_property VARCHAR(20) NOT NULL DEFAULT 'shapeName',
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
