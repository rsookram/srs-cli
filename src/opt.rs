use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use pico_args::Arguments;
use std::convert::Infallible;
use std::path::PathBuf;
use std::process;

/// Contains parsed command line arguments.
#[derive(Debug)]
pub struct Opt {
    /// The subcommand to run.
    pub command: Commands,

    /// The path of the database file. Defaults to srs.db.
    pub path: PathBuf,
}

#[derive(Debug)]
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

impl Opt {
    /// Gets [Opt] from the command line arguments. Prints the error message
    /// and quits the program in case of failure.
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

    fn parse(mut args: Arguments) -> Result<Self> {
        let path = args
            .opt_value_from_os_str(["-p", "--path"], |p| Ok::<_, Infallible>(PathBuf::from(p)))?
            .unwrap_or_else(|| PathBuf::from("srs.db"));

        let subcommand = match args.subcommand()? {
            Some(s) => s,
            None => bail!("missing subcommand"),
        };

        let command = match subcommand.as_ref() {
            "add" => Commands::Add {
                deck_id: args.value_from_str("--deck-id")?,
            },
            "cards" => Commands::Cards,
            "create-deck" => Commands::CreateDeck {
                name: args.value_from_str("--name")?,
            },
            "decks" => Commands::Decks,
            "delete" => Commands::Delete {
                card_id: args.value_from_str("--card-id")?,
            },
            "delete-deck" => Commands::DeleteDeck {
                deck_id: args.value_from_str("--deck-id")?,
            },
            "edit" => Commands::Edit {
                card_id: args.value_from_str("--card-id")?,
            },
            "init" => Commands::Init,
            "int-mod" => Commands::IntMod {
                deck_id: args.value_from_str("--deck-id")?,
                modifier: args.value_from_str("--modifier")?,
            },
            "review" => Commands::Review,
            "stats" => Commands::Stats,
            "switch" => Commands::Switch {
                card_id: args.value_from_str("--card-id")?,
                deck_id: args.value_from_str("--deck-id")?,
            },
            _ => bail!("unknown subcommand"),
        };

        let remaining = args.finish();
        if remaining.is_empty() {
            Ok(Self { command, path })
        } else {
            Err(anyhow!(
                "found arguments which weren't expected: {remaining:?}"
            ))
        }
    }
}

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
    add
    cards
    create-deck
    decks
    delete
    delete-deck
    edit
    init
    int-mod
    review
    stats
    switch"#,
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
    );
}
