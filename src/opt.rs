//! Handling of command line arguments.

use pico_args::Arguments;
use srs_cli::error::Result;
use std::convert::Infallible;
use std::path::PathBuf;
use std::process;

/// A parsed representation of command line arguments.
#[derive(Debug)]
pub struct Opt {
    /// The [Subcommand] to run.
    pub subcommand: Subcommand,

    /// The path of the database file. Defaults to ./srs.db.
    pub path: PathBuf,
}

/// The subcommand to run.
#[derive(Debug)]
pub enum Subcommand {
    /// Create a new card.
    Add,
    /// List all cards.
    List,
    /// Delete the card with the given ID.
    Delete { card_id: u16 },
    /// Edit the contents of the card with the given ID.
    Edit { card_id: u16 },
    /// Review cards that are scheduled for review.
    Review,
    /// Output statistics of reviews.
    Stats,
}

impl Opt {
    /// Gets [Opt] from the command line arguments. Prints the error message and quits the program
    /// in case of failure.
    pub fn from_args() -> Self {
        let mut args = Arguments::from_env();

        if args.contains(["-h", "--help"]) {
            print_help();
            process::exit(0);
        }

        if args.contains(["-V", "--version"]) {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            process::exit(0);
        }

        Self::parse(args).unwrap_or_else(|e| {
            eprintln!("error: {e}");
            process::exit(1);
        })
    }

    /// Parses [Arguments] into [Opt], resulting in an error when unexpected arguments are
    /// provided, or expected arguments are missing.
    fn parse(mut args: Arguments) -> Result<Self> {
        let path = args
            .opt_value_from_os_str(["-p", "--path"], |p| Ok::<_, Infallible>(PathBuf::from(p)))?
            .unwrap_or_else(|| PathBuf::from("srs.db"));

        let subcommand = match args.subcommand()? {
            Some(s) => s,
            None => return Err("missing subcommand".into()),
        };

        let subcommand = match subcommand.as_ref() {
            "add" => Subcommand::Add,
            "list" => Subcommand::List,
            "delete" => Subcommand::Delete {
                card_id: args.value_from_str("--card-id")?,
            },
            "edit" => Subcommand::Edit {
                card_id: args.value_from_str("--card-id")?,
            },
            "review" => Subcommand::Review,
            "stats" => Subcommand::Stats,
            _ => return Err(format!("unknown subcommand `{subcommand}`").into()),
        };

        let remaining = args.finish();
        if remaining.is_empty() {
            Ok(Self { subcommand, path })
        } else {
            Err(format!("found arguments which weren't expected: {remaining:?}").into())
        }
    }
}

/// Print the help output requested by the -h or --help flag.
fn print_help() {
    println!(
        r#"{name} {version}
Spaced repetition at the command line

USAGE:
    {name} [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -p, --path <PATH>    The path of the database file [default: srs.db]

SUBCOMMANDS:
    add            Create a new card
    list           List all cards
    delete         Delete a card
    edit           Edit the contents of a card
    review         Review cards that are scheduled for review
    stats          View statistics of reviews"#,
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
    );
}
