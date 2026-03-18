use std::net::SocketAddr;
use std::time::Duration;

use clap::{Parser, Subcommand};
use simplesolat_api::models::db::connect_db;
use simplesolat_api::routes::create_app_router;
use simplesolat_api::service;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "simplesolat-api")]
#[command(about = "SimpleSolat prayer times API and sync tool")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the API server
    Serve,
    /// Sync prayer times data from upstream sources
    Sync {
        /// Source to sync: jakim, muis, equran, kheu (omit for all)
        source: Option<String>,
        /// Run sync in a loop with the given interval (e.g. 6h, 30m, 1d)
        #[arg(long)]
        r#loop: Option<String>,
    },
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    let (num, unit) = s.split_at(
        s.find(|c: char| !c.is_ascii_digit())
            .unwrap_or(s.len()),
    );
    let num: u64 = num.parse().map_err(|_| format!("invalid number: {}", num))?;
    match unit {
        "s" | "" => Ok(Duration::from_secs(num)),
        "m" => Ok(Duration::from_secs(num * 60)),
        "h" => Ok(Duration::from_secs(num * 3600)),
        "d" => Ok(Duration::from_secs(num * 86400)),
        _ => Err(format!("unknown unit: {}, use s/m/h/d", unit)),
    }
}

async fn run_sync(source: &Option<String>, conn: &mut diesel::PgConnection) {
    service::zones::upsert_zones_from_data(conn);

    match source.as_deref() {
        Some("jakim") => {
            println!("Sync prayer times data from Jakim");
            service::prayer_times::sync_prayer_times_from_jakim(conn).await;
        }
        Some("muis") => {
            println!("Sync prayer times data from MUIS");
            service::prayer_times::sync_prayer_times_from_muis(conn).await;
        }
        Some("equran") => {
            println!("Sync prayer times data from EQuran.id");
            service::prayer_times::sync_prayer_times_from_equran(conn).await;
        }
        Some("kheu") => {
            println!("Sync prayer times data from KHEU");
            service::prayer_times::sync_prayer_times_from_kheu(conn).await;
        }
        None => {
            println!("Sync prayer times data from Jakim");
            service::prayer_times::sync_prayer_times_from_jakim(conn).await;

            println!("Sync prayer times data from MUIS");
            service::prayer_times::sync_prayer_times_from_muis(conn).await;

            println!("Sync prayer times data from EQuran.id");
            service::prayer_times::sync_prayer_times_from_equran(conn).await;

            println!("Sync prayer times data from KHEU");
            service::prayer_times::sync_prayer_times_from_kheu(conn).await;
        }
        Some(other) => {
            eprintln!(
                "Unknown source: {}. Use 'jakim', 'muis', 'equran', 'kheu', or omit for all.",
                other
            );
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "simplesolat_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Serve) => {
            let router = create_app_router().await;

            let port = std::env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse::<u16>()
                .expect("PORT must be a valid number");

            let addr = SocketAddr::from(([0, 0, 0, 0], port));
            tracing::info!("Starting server on {}", addr);

            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, router).await.unwrap();
        }
        Some(Commands::Sync {
            ref source,
            ref r#loop,
        }) => {
            let db_pool = connect_db();
            let mut conn = db_pool.get().unwrap();

            match r#loop {
                Some(interval_str) => {
                    let interval = parse_duration(interval_str).unwrap_or_else(|e| {
                        eprintln!("Invalid loop interval: {}", e);
                        std::process::exit(1);
                    });
                    println!("Running sync in loop mode (interval: {}s)", interval.as_secs());
                    loop {
                        run_sync(source, &mut conn).await;
                        println!("Sleeping for {}s until next sync...", interval.as_secs());
                        tokio::time::sleep(interval).await;
                    }
                }
                None => {
                    run_sync(source, &mut conn).await;
                }
            }
        }
    }
}
