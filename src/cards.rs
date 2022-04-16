use anyhow::Result;
use rusqlite::Connection;
use rusqlite::OpenFlags;
use std::path::PathBuf;

#[derive(Debug)]
struct Card {
    id: u64,
    front: String,
}

pub fn run(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let mut stmt = conn.prepare(
        "
        SELECT id, front
        FROM Card
        ORDER BY creationTimestamp DESC;
        ",
    )?;

    let card_iter = stmt.query_map([], |row| {
        Ok(Card {
            id: row.get(0)?,
            front: row.get(1)?,
        })
    })?;

    for card in card_iter {
        let card = card?;
        println!("{} {}", card.id, card.front.replace('\n', ""));
    }

    Ok(())
}
