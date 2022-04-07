use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;

pub fn run(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open(db_path)?;

    conn.execute_batch(include_str!("schema.sql"))?;

    Ok(())
}
