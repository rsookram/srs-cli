use crate::editor;
use crate::select;
use anyhow::Result;
use rusqlite::params;
use rusqlite::Connection;
use skim::prelude::*;
use skim::SkimItem;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct Card {
    id: u64,
    front: String,
    back: String,
}

impl SkimItem for Card {
    fn text(&self) -> std::borrow::Cow<str> {
        Cow::from(&self.front)
    }
}

pub fn run(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open(db_path)?;

    let mut stmt = conn.prepare(
        "
        SELECT id, front, back
        FROM Card
        ORDER BY creationTimestamp DESC;
        ",
    )?;

    let cards: Vec<Card> = stmt
        .query_map([], |row| {
            Ok(Card {
                id: row.get(0)?,
                front: row.get(1)?,
                back: row.get(2)?,
            })
        })?
        .filter_map(|card| card.ok())
        .collect();

    if let Some(card) = select::skim(&cards) {
        let (front, back) = editor::edit(&card.front, &card.back)?;

        conn.execute(
            "UPDATE Card SET front=?, back=? WHERE id=?",
            params![front, back, card.id],
        )?;
    }

    Ok(())
}
