mod app;
mod opt;

use crate::app::App;
use anyhow::Result;
use srs_cli::Srs;
use std::io;

fn main() -> Result<()> {
    let opt = opt::Opt::from_args();

    let srs = Srs::open(&opt.path)?;

    let stdout = std::io::stdout();
    let stdout = stdout.lock();

    let mut app = App::new(srs, stdout);

    use opt::Commands::*;

    let result = match &opt.command {
        Add { deck_id } => app.add(*deck_id),

        Cards => app.cards(),

        CreateDeck { name } => app.create_deck(name),

        Decks => app.decks(),

        Delete { card_id } => app.delete(*card_id),

        DeleteDeck { deck_id } => app.delete_deck(*deck_id),

        Edit { card_id } => app.edit(*card_id),

        Init => app.init(),

        IntMod { deck_id, modifier } => app.int_mod(*deck_id, *modifier),

        Review => app.review(),

        Stats => app.stats(),

        Switch { card_id, deck_id } => app.switch(*card_id, *deck_id),
    };

    if let Err(err) = result {
        match err.downcast_ref::<io::Error>() {
            Some(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
            _ => Err(err),
        }
    } else {
        result
    }
}
