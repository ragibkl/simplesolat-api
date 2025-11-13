use simplesolat_api::{models::db::connect_db, *};

#[tokio::main]
async fn main() {
    println!("Connecting to postgres");
    // let conn = &mut establish_connection();
    let db_pool = connect_db();
    let mut conn = db_pool.get().unwrap();

    println!("Upsert zones data");
    service::zones::upsert_zones_from_data(&mut conn);

    println!("Sync prayer times data from Jakim");
    service::prayer_times::sync_prayer_times_from_jakim(&mut conn).await;
}
