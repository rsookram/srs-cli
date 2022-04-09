use crate::select;
use anyhow::Result;
use dialoguer::Confirm;
use rusqlite::config::DbConfig;
use rusqlite::Connection;
use skim::prelude::*;
use skim::SkimItem;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct Card {
    id: u64,
    front: String,
    deck_name: String,
}

impl SkimItem for Card {
    fn text(&self) -> std::borrow::Cow<str> {
        Cow::from(format!("[{}] {}", self.deck_name, self.front))
    }
}

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

    let cards = get_cards(&conn)?;

    if let Some(card) = select::skim(&cards) {
        let mut decks = get_decks(&conn)?;

        decks.retain(|deck| deck.name != card.deck_name);

        if let Some(deck) = select::skim(&decks) {
            if Confirm::new()
                .with_prompt(format!(
                    "Are you sure you want to switch '{}' to {}?",
                    card.front, deck.name
                ))
                .interact()?
            {
                conn.execute(
                    "UPDATE Card SET deckId = ? WHERE id = ?",
                    [deck.id, card.id],
                )?;
                println!("... switched.");
            }
        }
    }

    Ok(())
}

fn get_cards(conn: &Connection) -> Result<Vec<Card>> {
    let mut stmt = conn.prepare(
        "
        SELECT Card.id, Card.front, Deck.name
        FROM Card
        JOIN Deck ON Card.deckId = Deck.id
        ORDER BY Card.creationTimestamp DESC
        ",
    )?;

    let cards: Vec<Card> = stmt
        .query_map([], |row| {
            Ok(Card {
                id: row.get(0)?,
                front: row.get(1)?,
                deck_name: row.get(2)?,
            })
        })?
        .filter_map(|card| card.ok())
        .collect();

    Ok(cards)
}

fn get_decks(conn: &Connection) -> Result<Vec<Deck>> {
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

    Ok(decks)
}
