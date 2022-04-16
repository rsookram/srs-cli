use crate::editor;
use anyhow::Result;
use rusqlite::params;
use rusqlite::Connection;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct Card {
    front: String,
    back: String,
}

pub fn run(db_path: &PathBuf, card_id: u64) -> Result<()> {
    let conn = Connection::open(db_path)?;

    let card = conn.query_row(
        "SELECT front, back FROM Card WHERE id = ?",
        [card_id],
        |row| {
            Ok(Card {
                front: row.get(0)?,
                back: row.get(1)?,
            })
        },
    )?;

    let (front, back) = editor::edit(&card.front, &card.back)?;

    conn.execute(
        "UPDATE Card SET front=?, back=? WHERE id=?",
        params![front, back, card_id],
    )?;

    Ok(())
}
