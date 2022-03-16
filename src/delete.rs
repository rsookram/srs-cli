use crate::select;
use anyhow::Result;
use dialoguer::Confirm;
use rusqlite::Connection;
use skim::prelude::*;
use skim::SkimItem;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct Card {
    id: u64,
    front: String,
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
        SELECT id, front
        FROM Card
        JOIN Schedule ON Card.id = Schedule.cardId
        ORDER BY isLeech DESC, creationTimestamp DESC;
        ",
    )?;

    let cards: Vec<Card> = stmt
        .query_map([], |row| {
            Ok(Card {
                id: row.get(0)?,
                front: row.get(1)?,
            })
        })?
        .filter_map(|card| card.ok())
        .collect();

    if let Some(Card { id, front }) = select::skim(&cards) {
        if Confirm::new()
            .with_prompt(format!(
                "Are you sure you want to delete '{}'",
                front.replace('\n', " ")
            ))
            .interact()?
        {
            conn.execute("DELETE FROM Card WHERE id = ?", [id])?;
            println!("... deleted.");
        }
    }

    Ok(())
}
