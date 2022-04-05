use crate::select;
use anyhow::bail;
use anyhow::Result;
use dialoguer::Input;
use rusqlite::params;
use rusqlite::Connection;
use skim::prelude::Cow;
use skim::SkimItem;
use std::path::PathBuf;

const MIN_INTERVAL_MODIFIER: u16 = 50;

#[derive(Debug, Clone)]
struct Deck {
    id: u64,
    name: String,
    interval_modifier: u16,
}

impl SkimItem for Deck {
    fn text(&self) -> std::borrow::Cow<str> {
        Cow::from(format!("{} - {}", &self.interval_modifier, &self.name))
    }
}

pub fn run(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open(db_path)?;

    let decks = get_decks(&conn)?;

    if let Some(deck) = select::skim(&decks) {
        let new_modifier: String = Input::new()
            .with_prompt("Enter new interval modifier")
            .with_initial_text(format!("{}", deck.interval_modifier))
            .validate_with(|input: &String| -> Result<()> {
                let parsed = input.parse::<u16>()?;
                if parsed < MIN_INTERVAL_MODIFIER {
                    bail!(format!("must be > {MIN_INTERVAL_MODIFIER}, given {parsed}"));
                }

                Ok(())
            })
            .interact_text()?;

        let new_modifier = new_modifier.parse::<u16>().expect("validated on input");

        conn.execute(
            "
            UPDATE Deck
            SET intervalModifier = ?
            WHERE id = ?
            ",
            params![new_modifier, deck.id],
        )?;

        println!("Set interval modifier for {} to {new_modifier}", &deck.name);
    }

    Ok(())
}

fn get_decks(conn: &Connection) -> Result<Vec<Deck>> {
    let mut stmt = conn.prepare(
        "
        SELECT id, name, intervalModifier
        FROM Deck
        ORDER BY name
        ",
    )?;

    let decks = stmt
        .query_map([], |row| {
            Ok(Deck {
                id: row.get(0)?,
                name: row.get(1)?,
                interval_modifier: row.get(2)?,
            })
        })?
        .filter_map(|deck| deck.ok())
        .collect();

    Ok(decks)
}
