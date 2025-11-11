use std::env;

use diesel::prelude::*;
use dotenvy::dotenv;
use simplesolat_api::*;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

#[tokio::main]
async fn main() {
    println!("Connecting to postgres");
    let conn = &mut establish_connection();

    println!("Upsert zones data");
    service::zones::upsert_zones_from_data(conn);

    println!("Sync prayer times data from Jakim");
    service::prayer_times::sync_prayer_times_from_jakim(conn).await;
}
