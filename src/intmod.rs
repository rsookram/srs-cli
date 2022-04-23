use crate::srs::Srs;
use anyhow::Result;

pub fn run(mut srs: Srs, deck_id: u64, modifier: u16) -> Result<()> {
    let name: String = srs.get_deck(deck_id)?.name;

    srs.update_interval_modifier(deck_id, modifier)?;

    println!("Set interval modifier for {name} to {modifier}");

    Ok(())
}
