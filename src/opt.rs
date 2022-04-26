use clap::Parser;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Commands,

    #[clap(short, long, global = true, parse(from_os_str), default_value_os_t = PathBuf::from("srs.db"))]
    pub path: PathBuf,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Add { deck_id: u64 },
    Cards,
    CreateDeck { name: String },
    Decks,
    Delete { card_id: u64 },
    DeleteDeck { deck_id: u64 },
    Edit { card_id: u64 },
    Init,
    IntMod { deck_id: u64, modifier: u16 },
    Review,
    Stats,
    Switch { card_id: u64, deck_id: u64 },
}
