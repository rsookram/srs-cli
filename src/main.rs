use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use srs_cli::add;
use srs_cli::cards;
use srs_cli::createdeck;
use srs_cli::decks;
use srs_cli::delete;
use srs_cli::deletedeck;
use srs_cli::edit;
use srs_cli::init;
use srs_cli::intmod;
use srs_cli::review;
use srs_cli::stats;
use srs_cli::switch;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Commands,

    #[clap(short, long, global = true, parse(from_os_str), default_value_os_t = PathBuf::from("srs.db"))]
    path: PathBuf,
}

#[derive(Debug, Subcommand)]
enum Commands {
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

fn main() -> Result<()> {
    let args = Args::parse();

    match &args.command {
        Commands::Add { deck_id } => add::run(&args.path, *deck_id),
        Commands::Cards => cards::run(&args.path),
        Commands::CreateDeck { name } => createdeck::run(&args.path, name),
        Commands::Decks => decks::run(&args.path),
        Commands::Delete { card_id } => delete::run(&args.path, *card_id),
        Commands::DeleteDeck { deck_id } => deletedeck::run(&args.path, *deck_id),
        Commands::Edit { card_id } => edit::run(&args.path, *card_id),
        Commands::Init => init::run(&args.path),
        Commands::IntMod { deck_id, modifier } => intmod::run(&args.path, *deck_id, *modifier),
        Commands::Review => review::run(&args.path),
        Commands::Stats => stats::run(&args.path),
        Commands::Switch { card_id, deck_id } => switch::run(&args.path, *card_id, *deck_id),
    }
}
