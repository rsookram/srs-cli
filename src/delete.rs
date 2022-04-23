use crate::srs::Srs;
use anyhow::Result;
use dialoguer::Confirm;

pub fn run(mut srs: Srs, card_id: u64) -> Result<()> {
    let front: String = srs.get_card(card_id)?.front;

    if Confirm::new()
        .with_prompt(format!(
            "Are you sure you want to delete '{}'",
            front.replace('\n', " ")
        ))
        .interact()?
    {
        srs.delete_card(card_id)?;
        println!("... deleted.");
    }

    Ok(())
}
