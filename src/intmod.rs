use anyhow::bail;
use anyhow::Result;
use rusqlite::params;
use rusqlite::Connection;
use std::path::PathBuf;

const MIN_INTERVAL_MODIFIER: u16 = 50;

pub fn run(db_path: &PathBuf, deck_id: u64, modifier: u16) -> Result<()> {
    if modifier < MIN_INTERVAL_MODIFIER {
        bail!(format!(
            "must be > {MIN_INTERVAL_MODIFIER}, given {modifier}"
        ));
    }

    let conn = Connection::open(db_path)?;

    let name: String = conn.query_row("SELECT name FROM Deck WHERE id = ?", [deck_id], |row| {
        row.get(0)
    })?;

    conn.execute(
        "
        UPDATE Deck
        SET intervalModifier = ?
        WHERE id = ?
        ",
        params![modifier, deck_id],
    )?;

    println!("Set interval modifier for {name} to {modifier}");

    Ok(())
}
