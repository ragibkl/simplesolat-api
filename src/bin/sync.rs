use simplesolat_api::{models::db::connect_db, *};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let source = args.get(1).map(|s| s.as_str());

    println!("Connecting to postgres");
    let db_pool = connect_db();
    let mut conn = db_pool.get().unwrap();

    println!("Upsert zones data");
    service::zones::upsert_zones_from_data(&mut conn);

    match source {
        Some("jakim") => {
            println!("Sync prayer times data from Jakim");
            service::prayer_times::sync_prayer_times_from_jakim(&mut conn).await;
        }
        Some("muis") => {
            println!("Sync prayer times data from MUIS");
            service::prayer_times::sync_prayer_times_from_muis(&mut conn).await;
        }
        None => {
            println!("Sync prayer times data from Jakim");
            service::prayer_times::sync_prayer_times_from_jakim(&mut conn).await;

            println!("Sync prayer times data from MUIS");
            service::prayer_times::sync_prayer_times_from_muis(&mut conn).await;
        }
        Some(other) => {
            eprintln!("Unknown source: {}. Use 'jakim', 'muis', or omit for all.", other);
            std::process::exit(1);
        }
    }
}
