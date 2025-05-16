//! Handling of command line arguments.

use srs_cli::error::Result;
use std::env::args_os;
use std::ffi::{OsStr, OsString};
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
        let args = Arguments::from_env();

        if args.contains("-h") || args.contains("--help") {
            print_help();
            process::exit(0);
        }

        if args.contains("-V") || args.contains("--version") {
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
    fn parse(args: Arguments) -> Result<Self> {
        let path = match args.opt_os_str("-p").or_else(|| args.opt_os_str("--path")) {
            Some(p) => PathBuf::from(p),
            None => PathBuf::from("srs.db"),
        };

        let subcommand = args
            .subcommand()
            .ok_or_else(|| "missing subcommand".to_string())?;

        let subcommand = match subcommand {
            "add" => Subcommand::Add,
            "list" => Subcommand::List,
            "delete" => Subcommand::Delete {
                card_id: args.value_as_u16("--card-id")?,
            },
            "edit" => Subcommand::Edit {
                card_id: args.value_as_u16("--card-id")?,
            },
            "review" => Subcommand::Review,
            "stats" => Subcommand::Stats,
            _ => return Err(format!("unknown subcommand `{subcommand}`").into()),
        };

        Ok(Self { subcommand, path })
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

#[derive(Debug)]
struct Arguments {
    args: Vec<OsString>,
}

impl Arguments {
    fn from_env() -> Self {
        Self {
            args: args_os().skip(1).collect(),
        }
    }

    fn contains(&self, key: &'static str) -> bool {
        self.args.iter().any(|arg| arg == key)
    }

    fn subcommand(&self) -> Option<&str> {
        let first = self.args.first()?.to_str()?;
        if first.starts_with('-') {
            return None;
        }

        Some(first)
    }

    fn opt_os_str(&self, key: &'static str) -> Option<&OsStr> {
        let idx = self.args.iter().position(|arg| arg == key)?;
        Some(self.args.get(idx + 1)?)
    }

    fn value_as_u16(&self, key: &'static str) -> Result<u16> {
        let value_str = self
            .opt_os_str(key)
            .ok_or_else(|| format!("missing option '{key}'"))?;

        let str = value_str
            .to_str()
            .ok_or_else(|| format!("invalid argument for '{key}' {value_str:?}"))?;
        Ok(str
            .parse::<u16>()
            .map_err(|err| format!("failed to parse value '{str}' for key '{key}': {err}"))?)
    }
}
