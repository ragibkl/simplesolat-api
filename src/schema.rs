// @generated automatically by Diesel CLI.

diesel::table! {
    prayer_times (id) {
        id -> Int8,
        #[max_length = 10]
        zone_code -> Nullable<Varchar>,
        date -> Nullable<Date>,
        imsak -> Nullable<Time>,
        fajr -> Nullable<Time>,
        syuruk -> Nullable<Time>,
        dhuhr -> Nullable<Time>,
        asr -> Nullable<Time>,
        maghrib -> Nullable<Time>,
        isha -> Nullable<Time>,
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
