use crate::select;
use anyhow::anyhow;
use anyhow::Result;
use chrono::Utc;
use rusqlite::params;
use rusqlite::Connection;
use scrawl;
use skim::prelude::*;
use skim::SkimItem;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct Deck {
    id: u64,
    name: String,
}

impl SkimItem for Deck {
    fn text(&self) -> std::borrow::Cow<str> {
        Cow::from(&self.name)
    }
}

pub fn run(db_path: &PathBuf) -> Result<()> {
    let mut conn = Connection::open(db_path)?;

    let decks = get_decks(&conn)?;

    if let Some(deck) = select::skim(&decks) {
        let (front, back) = read_card()?;

        let tx = conn.transaction()?;

        let now = Utc::now().timestamp_millis();

        let card_id: u64 = tx.query_row(
            "INSERT INTO Card(deckId, front, back, creationTimestamp) VALUES (?, ?, ?, ?) RETURNING *",
            params![deck.id, front, back, now],
            |row| row.get(0),
        )?;

        tx.execute(
            "INSERT INTO Schedule(cardId, scheduledForTimestamp, intervalDays) VALUES (?, ?, ?)",
            params![card_id, now, 0],
        )?;

        tx.commit()?;
    }

    Ok(())
}

fn get_decks(conn: &Connection) -> Result<Vec<Deck>> {
    let mut stmt = conn.prepare(
        "
        SELECT id, name
        FROM Deck
        ORDER BY name;
        ",
    )?;

    let decks = stmt
        .query_map([], |row| {
            Ok(Deck {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .filter_map(|deck| deck.ok())
        .collect();

    Ok(decks)
}

fn read_card() -> Result<(String, String)> {
    let divider = "----------";
    let template = format!("\n{divider}\n\n");

    let output = scrawl::with(&template)?;

    output
        .split_once(divider)
        .map(|(front, back)| (front.trim().to_string(), back.trim().to_string()))
        .ok_or(anyhow!("Missing divider between front and back of card"))
        .and_then(|(front, back)| {
            if front.is_empty() {
                Err(anyhow!("Can't add card without text on the front"))
            } else {
                Ok((front, back))
            }
        })
}
