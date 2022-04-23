use crate::srs::Srs;
use anyhow::Result;

pub fn run(srs: Srs) -> Result<()> {
    for card in srs.card_previews()? {
        let front = card.front.replace('\n', "");

        if card.is_leech {
            println!("[leech] {} {}", card.id, front);
        } else {
            println!("{} {}", card.id, front);
        }
    }

    Ok(())
}
