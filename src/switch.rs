use anyhow::Result;
use dialoguer::Confirm;
use rusqlite::config::DbConfig;
use rusqlite::Connection;
use std::path::PathBuf;

pub fn run(db_path: &PathBuf, card_id: u64, deck_id: u64) -> Result<()> {
    let conn = Connection::open(db_path)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;

    let (front, deck_name): (String, String) = conn.query_row(
        "
        SELECT
            (SELECT front from Card WHERE id = ?),
            (SELECT name from Deck WHERE id = ?)
        ",
        [card_id, deck_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    if Confirm::new()
        .with_prompt(format!(
            "Are you sure you want to switch '{}' to {}?",
            front, deck_name
        ))
        .interact()?
    {
        conn.execute(
            "UPDATE Card SET deckId = ? WHERE id = ?",
            [deck_id, card_id],
        )?;
        println!("... switched.");
    }

    Ok(())
}
