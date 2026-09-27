use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use simplesolat_api::api::data_repo::DEFAULT_BASE_URL;
use simplesolat_api::routes::create_app_router;
use simplesolat_api::store::DataStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "simplesolat-api")]
#[command(about = "SimpleSolat prayer times API, served from the simplesolat-data CDN")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the API server (the default)
    Serve,
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

fn env_duration(name: &str, default: &str) -> Duration {
    let value = std::env::var(name).unwrap_or_else(|_| default.to_string());
    parse_duration(&value).unwrap_or_else(|e| panic!("{} must be a duration like 1h: {}", name, e))
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
            let base_url =
                std::env::var("DATA_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
            let index_ttl = env_duration("ZONES_CACHE_TTL", "1d");
            let month_ttl = env_duration("PRAYER_TIMES_CACHE_TTL", "1d");
            let missing_ttl = env_duration("PRAYER_TIMES_MISSING_CACHE_TTL", "1h");
            tracing::info!(
                "data source {} (cached: zones {}s, prayer times {}s, unpublished months {}s)",
                base_url,
                index_ttl.as_secs(),
                month_ttl.as_secs(),
                missing_ttl.as_secs()
            );
            let store = Arc::new(DataStore::new(base_url, index_ttl, month_ttl, missing_ttl));
            let router = create_app_router(store);

            let port = std::env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse::<u16>()
                .expect("PORT must be a valid number");

            let addr = SocketAddr::from(([0, 0, 0, 0], port));
            tracing::info!("starting server on {}", addr);

            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, router).await.unwrap();
        }
    }
}
