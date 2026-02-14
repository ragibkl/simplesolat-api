use diesel::{
    PgConnection,
    r2d2::{self, ConnectionManager},
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

// Database connection pool type
pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

pub fn connect_db() -> DbPool {
    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or("postgres://user:password@localhost/simplesolat_db".to_string());

    // Create database connection pool
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let db_pool = r2d2::Pool::builder()
        .max_size(10) // Maximum 10 connections in the pool
        .build(manager)
        .expect("Failed to create database pool");

    // Test the connection and run migrations
    match db_pool.get() {
        Ok(mut conn) => {
            tracing::info!("✓ Database connection successful");
            conn.run_pending_migrations(MIGRATIONS)
                .expect("Failed to run database migrations");
            tracing::info!("✓ Database migrations up to date");
        }
        Err(e) => {
            tracing::error!("✗ Failed to connect to database: {}", e);
            panic!("Cannot start server without database connection");
        }
    }

    db_pool
}
