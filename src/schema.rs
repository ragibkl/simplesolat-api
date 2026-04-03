// @generated automatically by Diesel CLI.

diesel::table! {
    countries (code) {
        #[max_length = 2]
        code -> Varchar,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 100]
        source -> Varchar,
        geojson -> Text,
        mapping -> Text,
        #[max_length = 20]
        shape_property -> Varchar,
        created_at -> Timestamp,
    }
}

diesel::table! {
    prayer_times (id) {
        id -> Int8,
        #[max_length = 10]
        zone_code -> Varchar,
        date -> Date,
        imsak -> Time,
        fajr -> Time,
        syuruk -> Time,
        dhuhr -> Time,
        asr -> Time,
        maghrib -> Time,
        isha -> Time,
        created_at -> Timestamp,
    }
}

diesel::table! {
    zones (zone_code) {
        #[max_length = 10]
        zone_code -> Varchar,
        #[max_length = 100]
        state -> Varchar,
        location -> Text,
        created_at -> Timestamp,
        #[max_length = 2]
        country -> Varchar,
        #[max_length = 40]
        timezone -> Varchar,
    }
}

diesel::allow_tables_to_appear_in_same_query!(countries, prayer_times, zones,);
