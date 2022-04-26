mod opt;

use anyhow::anyhow;
use anyhow::Result;
use dialoguer::Confirm;
use rusqlite::Connection;
use srs_cli::Srs;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args = opt::Opt::from_args();

    let srs = Srs::open(&args.path)?;

    use opt::Commands::*;

    match &args.command {
        Add { deck_id } => add(srs, *deck_id),

        Cards => cards(srs),

        CreateDeck { name } => create_deck(srs, name),

        Decks => decks(srs),

        Delete { card_id } => delete(srs, *card_id),

        DeleteDeck { deck_id } => delete_deck(srs, *deck_id),

        Edit { card_id } => edit(srs, *card_id),

        Init => init(&args.path),

        IntMod { deck_id, modifier } => int_mod(srs, *deck_id, *modifier),

        Review => review::run(srs),

        Stats => stats::run(srs),

        Switch { card_id, deck_id } => switch(srs, *card_id, *deck_id),
    }
}

fn add(mut srs: Srs, deck_id: u64) -> Result<()> {
    let (front, back) = open_editor("", "")?;

    srs.create_card(deck_id, front, back)
}

fn cards(srs: Srs) -> Result<()> {
    for card in srs.card_previews()? {
        let front = card.front.replace('\n', "");

        if card.is_leech {
            println!("[leech] {} {}", card.id, front);
        } else {
            println!("{} {}", card.id, front);
        }
    }

    Ok(())
}

fn create_deck(mut srs: Srs, name: &str) -> Result<()> {
    srs.create_deck(name)?;

    println!("Created {name}");
    Ok(())
}

fn decks(srs: Srs) -> Result<()> {
    for deck in srs.decks()? {
        println!("{} {} - {}%", deck.id, deck.name, deck.interval_modifier);
    }

    Ok(())
}

fn delete(mut srs: Srs, card_id: u64) -> Result<()> {
    let front: String = srs.get_card(card_id)?.front;

    if Confirm::new()
        .with_prompt(format!(
            "Are you sure you want to delete '{}'",
            front.replace('\n', " ")
        ))
        .interact()?
    {
        srs.delete_card(card_id)?;
        println!("... deleted.");
    }

    Ok(())
}

fn delete_deck(mut srs: Srs, deck_id: u64) -> Result<()> {
    let name = srs.get_deck(deck_id)?.name;

    if Confirm::new()
        .with_prompt(format!("Are you sure you want to delete '{}'", name))
        .interact()?
    {
        srs.delete_deck(deck_id)?;
        println!("... deleted.");
    }

    Ok(())
}

fn edit(mut srs: Srs, card_id: u64) -> Result<()> {
    let card = srs.get_card(card_id)?;

    let (front, back) = open_editor(&card.front, &card.back)?;

    srs.update_card(card_id, front, back)
}

fn init(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open(db_path)?;

    conn.execute_batch(include_str!("schema.sql"))?;

    Ok(())
}

fn int_mod(mut srs: Srs, deck_id: u64, modifier: u16) -> Result<()> {
    let name: String = srs.get_deck(deck_id)?.name;

    srs.update_interval_modifier(deck_id, modifier)?;

    println!("Set interval modifier for {name} to {modifier}");

    Ok(())
}

fn switch(mut srs: Srs, card_id: u64, deck_id: u64) -> Result<()> {
    let front = srs.get_card(card_id)?.front;
    let deck_name = srs.get_deck(deck_id)?.name;

    if Confirm::new()
        .with_prompt(format!(
            "Are you sure you want to switch '{}' to {}?",
            front, deck_name
        ))
        .interact()?
    {
        srs.switch_deck(card_id, deck_id)?;
        println!("... switched.");
    }

    Ok(())
}

mod review {
    use anyhow::Result;
    use dialoguer::theme::Theme;
    use dialoguer::Confirm;
    use srs_cli::Card;
    use srs_cli::Srs;

    pub fn run(mut srs: Srs) -> Result<()> {
        let cards = srs.cards_to_review()?;

        for (deck_name, cards) in cards {
            let num_cards = cards.len();

            println!("{num_cards} cards to review in {deck_name}\n");

            let mut num_correct = 0;

            for card in cards {
                let is_correct = review_card(&card)?;
                if is_correct {
                    num_correct += 1;
                    srs.answer_correct(card.id)?;
                } else {
                    srs.answer_wrong(card.id)?;
                }

                println!();
            }

            println!("Answered {num_correct}/{num_cards} correctly")
        }

        println!("Finished review");

        Ok(())
    }

    fn review_card(card: &Card) -> Result<bool> {
        println!("{}\n", &card.front);

        Confirm::with_theme(&PlainPrompt)
            .with_prompt("Press enter to show answer")
            .default(true)
            .show_default(false)
            .report(false)
            .interact()?;

        println!("{}", "-".repeat(79));

        println!("{}\n", &card.back);

        Ok(Confirm::new().with_prompt("Correct?").interact()?)
    }

    struct PlainPrompt;

    impl Theme for PlainPrompt {
        /// Formats a confirm prompt without a trailing "[y/n]"
        fn format_confirm_prompt(
            &self,
            f: &mut dyn std::fmt::Write,
            prompt: &str,
            _default: Option<bool>,
        ) -> std::fmt::Result {
            write!(f, "{}", &prompt)
        }
    }
}

mod stats {
    use anyhow::Result;
    use srs_cli::DeckStats;
    use srs_cli::GlobalStats;
    use srs_cli::Srs;

    pub fn run(srs: Srs) -> Result<()> {
        let (global_stats, deck_stats) = srs.stats()?;

        print_global(&global_stats);

        deck_stats.iter().for_each(print_deck);

        Ok(())
    }

    fn print_global(stats: &GlobalStats) {
        let total = stats.active + stats.suspended + (stats.leech as u32);
        println!("{} / {} active", stats.active, total);

        if stats.leech > 0 {
            println!("{} leeches", stats.leech);
        }

        println!("Review tomorrow: {}", stats.for_review);
    }

    fn print_deck(stats: &DeckStats) {
        let total = stats.active + stats.suspended + (stats.leech as u32);
        println!("{}\n{} / {total} active", stats.name, stats.active);

        if stats.leech > 0 {
            println!("{} leeches", stats.leech);
        }

        let num_answered = stats.correct + stats.wrong;

        println!(
            "Past month accuracy: {:.0}% ({} / {num_answered})",
            if num_answered > 0 {
                stats.correct as f32 / num_answered as f32 * 100.0
            } else {
                100.0
            },
            stats.correct,
        );
    }
}

pub fn open_editor(front: &str, back: &str) -> Result<(String, String)> {
    let divider = "----------";
    let template = format!("{front}\n{divider}\n{back}\n");

    let output = scrawl::with(&template)?;

    output
        .split_once(divider)
        .map(|(front, back)| (front.trim().to_string(), back.trim().to_string()))
        .ok_or_else(|| anyhow!("Missing divider between front and back of card"))
        .and_then(|(front, back)| {
            if front.is_empty() {
                Err(anyhow!("Front of card can't be empty"))
            } else {
                Ok((front, back))
            }
        })
}
