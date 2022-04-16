use anyhow::Result;
use dialoguer::Confirm;
use rusqlite::config::DbConfig;
use rusqlite::Connection;
use std::path::PathBuf;

pub fn run(db_path: &PathBuf, card_id: u64) -> Result<()> {
    let conn = Connection::open(db_path)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;

    let front: String =
        conn.query_row("SELECT front FROM Card WHERE id = ?", [card_id], |row| {
            row.get(0)
        })?;

    if Confirm::new()
        .with_prompt(format!(
            "Are you sure you want to delete '{}'",
            front.replace('\n', " ")
        ))
        .interact()?
    {
        conn.execute("DELETE FROM Card WHERE id = ?", [card_id])?;
        println!("... deleted.");
    }

    Ok(())
}
