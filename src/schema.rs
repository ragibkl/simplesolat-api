// @generated automatically by Diesel CLI.

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
    }
}

diesel::allow_tables_to_appear_in_same_query!(prayer_times, zones,);
