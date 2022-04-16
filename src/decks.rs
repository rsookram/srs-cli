use anyhow::Result;
use rusqlite::Connection;
use rusqlite::OpenFlags;
use std::path::PathBuf;

#[derive(Debug)]
struct Deck {
    id: u64,
    name: String,
}

pub fn run(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let mut stmt = conn.prepare("SELECT id, name FROM Deck")?;

    let deck_iter = stmt.query_map([], |row| {
        Ok(Deck {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;

    for deck in deck_iter {
        let deck = deck?;
        println!("{} {}", deck.id, deck.name);
    }

    Ok(())
}
