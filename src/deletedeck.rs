use crate::srs::Srs;
use anyhow::Result;
use dialoguer::Confirm;

pub fn run(mut srs: Srs, deck_id: u64) -> Result<()> {
    let name = srs.get_deck(deck_id)?.name;

    if Confirm::new()
        .with_prompt(format!("Are you sure you want to delete '{}'", name))
        .interact()?
    {
        srs.delete_deck(deck_id)?;
        println!("... deleted.");
    }

    Ok(())
}
