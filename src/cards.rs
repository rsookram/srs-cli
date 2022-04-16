use anyhow::Result;
use rusqlite::Connection;
use rusqlite::OpenFlags;
use std::path::PathBuf;

#[derive(Debug)]
struct Card {
    id: u64,
    front: String,
    is_leech: bool,
}

pub fn run(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let mut stmt = conn.prepare(
        "
        SELECT id, front, isLeech
        FROM Card
        JOIN Schedule ON Card.id = Schedule.cardId
        ORDER BY isLeech DESC, creationTimestamp DESC;
        ",
    )?;

    let card_iter = stmt.query_map([], |row| {
        Ok(Card {
            id: row.get(0)?,
            front: row.get(1)?,
            is_leech: row.get(2)?,
        })
    })?;

    for card in card_iter {
        let card = card?;
        let front = card.front.replace('\n', "");

        if card.is_leech {
            println!("[leech] {} {}", card.id, front);
        } else {
            println!("{} {}", card.id, front);
        }
    }

    Ok(())
}
