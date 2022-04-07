use anyhow::bail;
use anyhow::Result;
use chrono::Utc;
use rusqlite::config::DbConfig;
use rusqlite::params;
use rusqlite::Connection;
use std::path::PathBuf;

pub fn run(db_path: &PathBuf, name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("deck name can't be empty");
    }

    let conn = Connection::open(db_path)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;

    let now = Utc::now().timestamp_millis();

    conn.execute(
        "INSERT INTO Deck(name, creationTimestamp) VALUES (?, ?)",
        params![name, now],
    )?;

    println!("Created {name}");

    Ok(())
}
