use crate::editor;
use anyhow::Result;
use rusqlite::config::DbConfig;
use rusqlite::params;
use rusqlite::Connection;
use std::path::PathBuf;
use time::OffsetDateTime;

pub fn run(db_path: &PathBuf, deck_id: u64) -> Result<()> {
    let mut conn = Connection::open(db_path)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;

    let (front, back) = editor::edit("", "")?;

    let tx = conn.transaction()?;

    let now: u64 = (OffsetDateTime::now_utc().unix_timestamp() * 1000)
        .try_into()
        .expect("valid timestamp");

    let card_id: u64 = tx.query_row(
        "INSERT INTO Card(deckId, front, back, creationTimestamp) VALUES (?, ?, ?, ?) RETURNING *",
        params![deck_id, front, back, now],
        |row| row.get(0),
    )?;

    tx.execute(
        "INSERT INTO Schedule(cardId, scheduledForTimestamp, intervalDays) VALUES (?, ?, ?)",
        params![card_id, now, 0],
    )?;

    tx.commit()?;

    Ok(())
}
