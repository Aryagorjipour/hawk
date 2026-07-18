use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::str::FromStr;

use crate::domain::{DomainError, DomainResult};

pub async fn connect(database_url: &str) -> DomainResult<SqlitePool> {
    ensure_sqlite_parent_dir(database_url)?;

    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(|e| DomainError::Persistence(e.to_string()))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .map_err(|e| DomainError::Persistence(format!("connect: {e}")))?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| DomainError::Persistence(format!("migrate: {e}")))?;

    Ok(pool)
}

fn ensure_sqlite_parent_dir(database_url: &str) -> DomainResult<()> {
    let path_part = database_url.strip_prefix("sqlite:").unwrap_or(database_url);
    let path_part = path_part.split('?').next().unwrap_or(path_part);
    if path_part == ":memory:" || path_part.is_empty() {
        return Ok(());
    }
    let path = Path::new(path_part);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DomainError::Persistence(format!("create db dir {}: {e}", parent.display()))
            })?;
        }
    }
    Ok(())
}
