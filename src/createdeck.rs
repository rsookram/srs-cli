use crate::srs::Srs;
use anyhow::Result;

pub fn run(mut srs: Srs, name: &str) -> Result<()> {
    srs.create_deck(name)?;

    println!("Created {name}");
    Ok(())
}
