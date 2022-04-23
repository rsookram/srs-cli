use crate::srs::Srs;
use anyhow::Result;
use dialoguer::Confirm;

pub fn run(mut srs: Srs, card_id: u64, deck_id: u64) -> Result<()> {
    let front = srs.get_card(card_id)?.front;
    let deck_name = srs.get_deck(deck_id)?.name;

    if Confirm::new()
        .with_prompt(format!(
            "Are you sure you want to switch '{}' to {}?",
            front, deck_name
        ))
        .interact()?
    {
        srs.switch_deck(card_id, deck_id)?;
        println!("... switched.");
    }

    Ok(())
}
