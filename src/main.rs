use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use srs_cli::delete;
use srs_cli::search;
use srs_cli::stats;
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
    Delete,
    Search,
    Stats,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match &args.command {
        Commands::Delete => delete::run(&args.path),
        Commands::Stats => stats::run(&args.path),
        Commands::Search => search::run(&args.path),
    }
}
