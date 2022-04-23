use crate::srs::Srs;
use anyhow::Result;

pub fn run(srs: Srs) -> Result<()> {
    for deck in srs.decks()? {
        println!("{} {} - {}%", deck.id, deck.name, deck.interval_modifier);
    }

    Ok(())
}
