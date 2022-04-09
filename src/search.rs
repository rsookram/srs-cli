use crate::select;
use anyhow::Result;
use rusqlite::Connection;
use rusqlite::OpenFlags;
use skim::prelude::*;
use skim::SkimItem;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct Card {
    front: String,
}

impl SkimItem for Card {
    fn text(&self) -> std::borrow::Cow<str> {
        Cow::from(&self.front)
    }
}

pub fn run(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let mut stmt = conn.prepare(
        "
        SELECT front
        FROM Card
        ORDER BY creationTimestamp DESC;
        ",
    )?;

    let cards: Vec<Card> = stmt
        .query_map([], |row| Ok(Card { front: row.get(0)? }))?
        .filter_map(|card| card.ok())
        .collect();

    if let Some(Card { front }) = select::skim(&cards) {
        println!("{}", front);
    }

    Ok(())
}
