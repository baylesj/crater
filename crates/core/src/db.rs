//! SQLite pool management and migration runner.
//!
//! `open` is the main entry point: it creates the database file if needed,
//! enables WAL mode and foreign keys, then runs any pending migrations.
//! `open_in_memory` is the test equivalent — same migrations, no file.

use std::path::Path;
use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::error::Result;

pub async fn open(data_dir: &Path) -> Result<SqlitePool> {
    tokio::fs::create_dir_all(data_dir)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create data dir {}: {e}", data_dir.display()))?;

    let db_path = data_dir.join("crater.db");
    let url = format!("sqlite://{}", db_path.display());

    let opts = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(16)
        .connect_with(opts)
        .await?;

    run_migrations(&pool).await?;
    Ok(pool)
}

/// In-memory database for tests. Same schema as the file-based version.
pub async fn open_in_memory() -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect("sqlite::memory:")
        .await?;

    run_migrations(&pool).await?;
    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| anyhow::anyhow!("migration failed: {e}"))?;
    Ok(())
}
