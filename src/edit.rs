use crate::editor;
use crate::srs::Srs;
use anyhow::Result;

pub fn run(mut srs: Srs, card_id: u64) -> Result<()> {
    let card = srs.get_card(card_id)?;

    let (front, back) = editor::edit(&card.front, &card.back)?;

    srs.update_card(card_id, front, back)
}
