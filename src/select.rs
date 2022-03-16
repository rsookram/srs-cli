use skim::prelude::*;

pub fn skim<T>(items: &[T]) -> Option<T>
where
    T: SkimItem + Clone,
{
    let skim_options = SkimOptionsBuilder::default()
        .height(Some("100%"))
        .build()
        .unwrap();

    let receiver = items_to_receiver(&items);

    if let Some(output) = Skim::run_with(&skim_options, Some(receiver))
        .filter(|out| !out.is_abort)
        .map(|out| out.selected_items)
    {
        if let Some(item) = output.first() {
            Some((**item).as_any().downcast_ref::<T>().unwrap().clone())
        } else {
            None
        }
    } else {
        None
    }
}

fn items_to_receiver<T>(items: &[T]) -> SkimItemReceiver
where
    T: SkimItem + Clone,
{
    let (tx_items, rx_items): (SkimItemSender, SkimItemReceiver) = unbounded();
    items.iter().for_each(|card| {
        let _ = tx_items.send(Arc::new(card.to_owned()));
    });
    drop(tx_items); // indicates that all items have been sent
    rx_items
}
