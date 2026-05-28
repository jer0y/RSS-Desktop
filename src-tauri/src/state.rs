use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use anyhow::{Context, Result};
use reqwest::Client;
use rusqlite::Connection;

use crate::db;

pub struct AppState {
    conn: Mutex<Connection>,
    pub http: Client,
}

impl AppState {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open(db_path).context("failed to open SQLite database")?;
        db::configure_connection(&conn)?;
        db::init_db(&conn)?;

        let http = build_http_client()?;

        Ok(Self {
            conn: Mutex::new(conn),
            http,
        })
    }

    pub fn conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock was poisoned"))
    }
}

pub fn build_http_client() -> Result<Client> {
    Client::builder()
        .user_agent("RSSDesktopWidget/0.1")
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(15))
        .pool_max_idle_per_host(2)
        .build()
        .context("failed to build HTTP client")
}
