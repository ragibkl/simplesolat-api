use diesel::{
    PgConnection,
    r2d2::{self, ConnectionManager},
};

// Database connection pool type
pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

pub fn connect_db() -> DbPool {
    // Get database URL from environment
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");

    // Create database connection pool
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let db_pool = r2d2::Pool::builder()
        .max_size(10) // Maximum 10 connections in the pool
        .build(manager)
        .expect("Failed to create database pool");

    // Test the connection
    match db_pool.get() {
        Ok(_) => tracing::info!("✓ Database connection successful"),
        Err(e) => {
            tracing::error!("✗ Failed to connect to database: {}", e);
            panic!("Cannot start server without database connection");
        }
    }

    db_pool
}
