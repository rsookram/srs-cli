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
    /// Create a new card, and add it to the deck with the given ID.
    Add { deck_id: u64 },
    /// List all cards in all decks.
    Cards,
    /// Create a new deck with the given name.
    CreateDeck { name: String },
    /// List all decks.
    Decks,
    /// Delete the card with the given ID.
    Delete { card_id: u64 },
    /// Delete the deck with the given ID, including all of its cards.
    DeleteDeck { deck_id: u64 },
    /// Edit the contents of the card with the given ID.
    Edit { card_id: u64 },
    /// Create an empty srs-cli database.
    Init,
    /// Set the interval modifier of the deck with the given ID.
    IntMod { deck_id: u64, modifier: u16 },
    /// Review cards that are scheduled for review.
    Review,
    /// Output statistics of decks and reviews.
    Stats,
    /// Move a card to the deck with the given ID.
    Switch { card_id: u64, deck_id: u64 },
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
            "add" => Subcommand::Add {
                deck_id: args.value_from_str("--deck-id")?,
            },
            "cards" => Subcommand::Cards,
            "create-deck" => Subcommand::CreateDeck {
                name: args.value_from_str("--name")?,
            },
            "decks" => Subcommand::Decks,
            "delete" => Subcommand::Delete {
                card_id: args.value_from_str("--card-id")?,
            },
            "delete-deck" => Subcommand::DeleteDeck {
                deck_id: args.value_from_str("--deck-id")?,
            },
            "edit" => Subcommand::Edit {
                card_id: args.value_from_str("--card-id")?,
            },
            "init" => Subcommand::Init,
            "int-mod" => Subcommand::IntMod {
                deck_id: args.value_from_str("--deck-id")?,
                modifier: args.value_from_str("--modifier")?,
            },
            "review" => Subcommand::Review,
            "stats" => Subcommand::Stats,
            "switch" => Subcommand::Switch {
                card_id: args.value_from_str("--card-id")?,
                deck_id: args.value_from_str("--deck-id")?,
            },
            _ => return Err("unknown subcommand `{subcommand}`".into()),
        };

        let remaining = args.finish();
        if remaining.is_empty() {
            Ok(Self { subcommand, path })
        } else {
            Err("found arguments which weren't expected: {remaining:?}".into())
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
    add            Create a new card, and add it to a deck
    cards          List all cards in all decks
    create-deck    Create a new deck
    decks          List all decks
    delete         Delete a card
    delete-deck    Delete a deck, including all of its cards
    edit           Edit the contents of a card
    init           Create an empty {name} database
    int-mod        Edit a deck's interval modifier
    review         Review cards that are scheduled for review
    stats          View statistics of decks and reviews
    switch         Move a card between decks"#,
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
    );
}
