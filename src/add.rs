use crate::editor;
use crate::select;
use anyhow::Result;
use chrono::Utc;
use rusqlite::config::DbConfig;
use rusqlite::params;
use rusqlite::Connection;
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
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;

    let decks = get_decks(&conn)?;

    if let Some(deck) = select::skim(&decks) {
        let (front, back) = editor::edit("", "")?;

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
