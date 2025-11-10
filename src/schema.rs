// @generated automatically by Diesel CLI.

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
