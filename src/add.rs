use crate::editor;
use crate::srs::Srs;
use anyhow::Result;

pub fn run(mut srs: Srs, deck_id: u64) -> Result<()> {
    let (front, back) = editor::edit("", "")?;

    srs.create_card(deck_id, front, back)
}
