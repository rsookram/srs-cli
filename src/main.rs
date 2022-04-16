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
    Add,
    Cards,
    CreateDeck { name: String },
    Decks,
    Delete,
    DeleteDeck,
    Edit,
    Init,
    IntMod,
    Review,
    Stats,
    Switch,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match &args.command {
        Commands::Add => add::run(&args.path),
        Commands::Cards => cards::run(&args.path),
        Commands::CreateDeck { name } => createdeck::run(&args.path, name),
        Commands::Decks => decks::run(&args.path),
        Commands::Delete => delete::run(&args.path),
        Commands::DeleteDeck => deletedeck::run(&args.path),
        Commands::Edit => edit::run(&args.path),
        Commands::Init => init::run(&args.path),
        Commands::IntMod => intmod::run(&args.path),
        Commands::Review => review::run(&args.path),
        Commands::Stats => stats::run(&args.path),
        Commands::Switch => switch::run(&args.path),
    }
}
