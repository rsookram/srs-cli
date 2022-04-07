use crate::select;
use anyhow::Result;
use dialoguer::Confirm;
use rusqlite::config::DbConfig;
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
    let conn = Connection::open(db_path)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;

    let mut stmt = conn.prepare("SELECT id, name FROM Deck")?;

    let decks: Vec<Deck> = stmt
        .query_map([], |row| {
            Ok(Deck {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .filter_map(|deck| deck.ok())
        .collect();

    if let Some(Deck { id, name }) = select::skim(&decks) {
        if Confirm::new()
            .with_prompt(format!("Are you sure you want to delete '{}'", name))
            .interact()?
        {
            conn.execute("DELETE FROM Deck WHERE id = ?", [id])?;
            println!("... deleted.");
        }
    }

    Ok(())
}
