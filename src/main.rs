mod opt;

use srs_cli::editor;
use srs_cli::error::Result;
use srs_cli::prompt;
use srs_cli::rand::Rng;
use srs_cli::Answer;
use srs_cli::Card;
use srs_cli::CardIndex;
use srs_cli::Srs;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::time::{Duration, SystemTime};

fn main() -> Result<()> {
    let opt = opt::Opt::from_args();

    let srs = match srs_cli::open(&opt.path) {
        Ok(s) => s,
        Err(e) => {
            if !opt.path.exists() {
                Srs::default()
            } else {
                return Err(e);
            }
        }
    };

    use opt::Subcommand::*;

    let result = match &opt.subcommand {
        Add => add_card(srs, &opt.path),
        List => list(srs),
        Delete { card_id } => delete_card(srs, &opt.path, *card_id),
        Edit { card_id } => edit_card(srs, &opt.path, *card_id),
        Review => review(srs, &opt.path),
        Stats => stats(srs),
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

fn add_card(srs: Srs, path: &Path) -> Result<()> {
    let (front, back) = open_editor("", "")?;

    srs_cli::add_card(srs, path, now_in_epoch_days(), front, back)
}

fn list(srs: Srs) -> Result<()> {
    let stdout = io::stdout().lock();
    let mut out = BufWriter::with_capacity(128 * 1024, stdout);

    writeln!(out, "  ID | Front")?;
    writeln!(out, "-----|--------")?;
    for (i, card) in srs.cards.iter().enumerate() {
        let front = srs_cli::card_front(card)?;
        writeln!(out, "{i:4} | {}", front)?;
    }

    Ok(())
}

fn delete_card(srs: Srs, path: &Path, idx: CardIndex) -> Result<()> {
    let card = srs
        .cards
        .get(usize::from(idx))
        .ok_or_else(|| format!("card {idx} doesn't exist"))?;
    let front = srs_cli::card_front(card)?;

    if prompt::binary(format!(
        "Are you sure you want to delete '{}'?",
        front.replace('\n', " ")
    ))? {
        srs_cli::delete_card(srs, path, idx)?;
        println!("... deleted.");
    }

    Ok(())
}

fn edit_card(srs: Srs, path: &Path, idx: CardIndex) -> Result<()> {
    let Card { front, back } = srs_cli::card(&srs, idx)?;

    let (edited_front, edited_back) = open_editor(&front, &back)?;

    srs_cli::edit_card(srs, path, idx, edited_front, edited_back)
}

fn review(srs: Srs, path: &Path) -> Result<()> {
    let mut card_indices = srs_cli::cards_to_review(&srs, now_in_epoch_days());
    let num_cards = card_indices.len();

    println!("{} cards to review", num_cards);

    let mut rng = Rng::new();
    rng.shuffle(&mut card_indices);

    let mut answers = Vec::with_capacity(card_indices.len());

    let mut num_correct = 0;
    for i in card_indices {
        let card = srs_cli::card(&srs, i)?;

        let is_correct = review_card(&card)?;
        if is_correct {
            num_correct += 1;
        }

        answers.push(Answer {
            card_index: i,
            is_correct,
        });

        println!();
    }

    println!("Finished review. Answered {num_correct}/{num_cards} correctly.");

    srs_cli::apply_answers(srs, path, now_in_epoch_days(), &mut answers)
}

fn review_card(card: &Card) -> Result<bool> {
    println!("{}\n", card.front);

    prompt::enter("Press enter to show answer")?;
    println!("{}", "─".repeat(39));

    println!("{}\n", card.back);

    prompt::binary("Correct?")
}

fn stats(srs: Srs) -> Result<()> {
    let stdout = io::stdout().lock();
    let mut out = BufWriter::with_capacity(8 * 1024, stdout);

    writeln!(out, " Days |  ✓  |  ✕  ")?;
    writeln!(out, "------|-----|-----")?;
    for (i, stat) in srs.stats.iter().enumerate() {
        writeln!(out, "  {i:3} | {:3} | {:3}", stat.correct, stat.wrong)?;
    }

    Ok(())
}

fn open_editor(front: &str, back: &str) -> Result<(String, String)> {
    let divider = "----------";
    let template = format!("{front}\n{divider}\n{back}\n");

    let output = editor::edit(&template)?;

    output
        .split_once(divider)
        .ok_or_else(|| "Missing divider between front and back of card".into())
        .map(|(front, back)| (front.trim().to_string(), back.trim().to_string()))
        .and_then(|(front, back)| {
            if front.is_empty() {
                Err("Front of card can't be empty".into())
            } else {
                Ok((front, back))
            }
        })
}

fn now_in_epoch_days() -> u16 {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("after unix epoch");

    now.div_duration_f32(Duration::from_secs(24 * 60 * 60)) as u16
}
