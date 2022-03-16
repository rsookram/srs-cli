use anyhow::Result;
use rusqlite::Connection;
use rusqlite::OpenFlags;
use skim::prelude::*;
use skim::SkimItem;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct Card {
    front: String,
}

impl SkimItem for Card {
    fn text(&self) -> std::borrow::Cow<str> {
        Cow::from(&self.front)
    }
}

pub fn run(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let mut stmt = conn.prepare(
        "
        SELECT front
        FROM Card
        JOIN Schedule ON Card.id = Schedule.cardId
        ORDER BY creationTimestamp DESC;
        ",
    )?;

    let cards: Vec<Card> = stmt
        .query_map([], |row| Ok(Card { front: row.get(0)? }))?
        .filter_map(|card| card.ok())
        .collect();

    let skim_options = SkimOptionsBuilder::default()
        .height(Some("100%"))
        .build()
        .unwrap();

    let receiver = cards_to_receiver(&cards);

    if let Some(output) = Skim::run_with(&skim_options, Some(receiver))
        .filter(|out| !out.is_abort)
        .map(|out| out.selected_items)
    {
        if let Some(item) = output.first() {
            let selected_card = (**item).as_any().downcast_ref::<Card>().unwrap();
            println!("{}", selected_card.front);
        }
    }

    Ok(())
}

fn cards_to_receiver(items: &[Card]) -> SkimItemReceiver {
    let (tx_items, rx_items): (SkimItemSender, SkimItemReceiver) = unbounded();
    items.iter().for_each(|card| {
        let _ = tx_items.send(Arc::new(card.to_owned()));
    });
    drop(tx_items); // indicates that all items have been sent
    rx_items
}
