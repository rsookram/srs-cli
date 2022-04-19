use anyhow::bail;
use anyhow::Result;
use rusqlite::config::DbConfig;
use rusqlite::params;
use rusqlite::Connection;
use std::path::PathBuf;
use time::OffsetDateTime;

pub fn run(db_path: &PathBuf, name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("deck name can't be empty");
    }

    let conn = Connection::open(db_path)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;

    let now: u64 = (OffsetDateTime::now_utc().unix_timestamp() * 1000)
        .try_into()
        .expect("valid timestamp");

    conn.execute(
        "INSERT INTO Deck(name, creationTimestamp) VALUES (?, ?)",
        params![name, now],
    )?;

    println!("Created {name}");

    Ok(())
}
