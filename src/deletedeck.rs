use anyhow::Result;
use dialoguer::Confirm;
use rusqlite::config::DbConfig;
use rusqlite::Connection;
use std::path::PathBuf;

pub fn run(db_path: &PathBuf, deck_id: u64) -> Result<()> {
    let conn = Connection::open(db_path)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;

    let name: String = conn.query_row("SELECT name FROM Deck WHERE id = ?", [deck_id], |row| {
        row.get(0)
    })?;

    if Confirm::new()
        .with_prompt(format!("Are you sure you want to delete '{}'", name))
        .interact()?
    {
        conn.execute("DELETE FROM Deck WHERE id = ?", [deck_id])?;
        println!("... deleted.");
    }

    Ok(())
}
