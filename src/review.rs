use anyhow::Result;
use chrono::Duration;
use chrono::Local;
use chrono::Utc;
use dialoguer::theme::Theme;
use dialoguer::Confirm;
use rand::seq::SliceRandom;
use rand::Rng;
use rusqlite::config::DbConfig;
use rusqlite::params;
use rusqlite::types::Null;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use std::path::PathBuf;

const AUTO_SUSPEND_INTERVAL: u16 = 365;
const WRONG_ANSWERS_FOR_LEECH: u16 = 4;
const FUZZ_FACTOR: f64 = 0.05; // TODO: consider increasing
const WRONG_ANSWER_PENALTY: f64 = 0.7;

#[derive(Debug)]
struct Card {
    deck_name: String,
    id: u64,
    front: String,
    back: String,
}

pub fn run(db_path: &PathBuf) -> Result<()> {
    let mut conn = Connection::open(db_path)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;

    let cards = get_cards(&mut conn)?;

    println!("{} cards to review\n", cards.len());

    let cards = cards_to_review(cards);

    for (deck_name, cards) in cards {
        let num_cards = cards.len();

        println!("{num_cards} cards to review in {deck_name}\n");

        let mut num_correct = 0;

        for card in cards {
            let is_correct = review_card(&card)?;
            if is_correct {
                num_correct += 1;
                answer_correct(&mut conn, card.id)?;
            } else {
                answer_wrong(&mut conn, card.id)?;
            }

            println!();
        }

        println!("Answered {num_correct}/{num_cards} correctly")
    }

    println!("Finished review");

    Ok(())
}

fn get_cards(conn: &mut Connection) -> Result<Vec<Card>> {
    let mut stmt = conn.prepare(
        "
        SELECT Deck.name, Card.id, Card.front, Card.back
        FROM Card JOIN Schedule ON Card.id = Schedule.cardId JOIN Deck ON Card.deckId = Deck.id
        WHERE isLeech = 0 AND scheduledForTimestamp < ?
        ORDER BY Card.deckId, scheduledForTimestamp
        ",
    )?;

    let start_of_tomorrow = Local::today().succ().and_hms(0, 0, 0).timestamp_millis();

    let card_iter = stmt.query_map([start_of_tomorrow], |row| {
        Ok(Card {
            deck_name: row.get(0)?,
            id: row.get(1)?,
            front: row.get(2)?,
            back: row.get(3)?,
        })
    })?;

    let mut cards = vec![];
    for card in card_iter {
        cards.push(card?);
    }

    Ok(cards)
}

fn cards_to_review(cards: Vec<Card>) -> Vec<(String, Vec<Card>)> {
    if cards.is_empty() {
        return vec![];
    }

    let mut rng = rand::thread_rng();

    let mut cards_by_deck = vec![];

    let mut current_deck = cards[0].deck_name.clone();
    let mut current_cards = vec![];
    for card in cards {
        if card.deck_name != current_deck {
            let deck_name = std::mem::take(&mut current_deck);
            let mut cards = std::mem::take(&mut current_cards);

            cards.shuffle(&mut rng);
            cards_by_deck.push((deck_name, cards));

            current_deck = card.deck_name.clone();
        }

        current_cards.push(card);
    }

    current_cards.shuffle(&mut rng);
    cards_by_deck.push((current_deck, current_cards));

    cards_by_deck
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

fn answer_correct(conn: &mut Connection, card_id: u64) -> Result<()> {
    let now_date_time = Utc::now();
    let now = now_date_time.timestamp_millis();

    let tx = conn.transaction()?;

    let interval_days: u16 = tx
        .query_row(
            "SELECT intervalDays FROM Schedule WHERE cardId = ?",
            [card_id],
            |row| row.get(0),
        )
        .optional()?
        .expect("card is not suspended");

    tx.execute(
        "INSERT INTO Answer(cardId, isCorrect, timestamp) VALUES (?, ?, ?)",
        params![card_id, true, now],
    )?;

    let was_correct: Option<bool> = tx
        .query_row(
            "SELECT isCorrect FROM Answer WHERE cardId = ? ORDER BY timestamp DESC LIMIT 1",
            [card_id],
            |row| row.get(0),
        )
        .optional()?;

    let interval_modifier: u16 = tx.query_row(
        "SELECT intervalModifier FROM Deck JOIN Card ON Deck.id = Card.deckId WHERE Card.id = ?",
        [card_id],
        |row| row.get(0),
    )?;

    let num_days = next_interval(interval_days, was_correct, interval_modifier);

    if num_days >= AUTO_SUSPEND_INTERVAL {
        tx.execute(
            "UPDATE Schedule SET scheduledForTimestamp = ?, intervalDays = ? WHERE cardId = ?",
            params![Null, Null, card_id],
        )?;
    } else {
        tx.execute(
            "UPDATE Schedule SET scheduledForTimestamp = ?, intervalDays = ? WHERE cardId = ?",
            params![
                (now_date_time + Duration::days(num_days.into())).timestamp_millis(),
                num_days,
                card_id
            ],
        )?;
    }

    tx.commit()?;

    Ok(())
}

fn answer_wrong(conn: &mut Connection, card_id: u64) -> Result<()> {
    let now = Utc::now().timestamp_millis();

    let tx = conn.transaction()?;

    tx.execute(
        "INSERT INTO Answer(cardId, isCorrect, timestamp) VALUES (?, ?, ?)",
        params![card_id, false, now],
    )?;

    tx.execute(
        "UPDATE Schedule SET scheduledForTimestamp = ? WHERE cardId = ?",
        params![now, card_id],
    )?;

    let num_wrong: u16 = tx.query_row(
        "SELECT COUNT(*) FROM Answer WHERE cardId = ? AND isCorrect = 0",
        [card_id],
        |row| row.get(0),
    )?;

    if num_wrong >= WRONG_ANSWERS_FOR_LEECH {
        tx.execute(
            "UPDATE Schedule SET isLeech = 1 WHERE cardId = ?",
            [card_id],
        )?;
    }

    tx.commit()?;

    Ok(())
}

fn next_interval(previous_interval: u16, was_correct: Option<bool>, interval_modifier: u16) -> u16 {
    match previous_interval {
        // Newly added card answered correctly
        0 => 1,
        // First review
        1 => 4,
        _ => {
            let was_correct = was_correct.expect("previously answered the card");

            if was_correct {
                // Previous answer was correct
                let mut next =
                    (previous_interval as f64 * 2.5 * (interval_modifier as f64 / 100.0)) as u16;

                let mut rng = rand::thread_rng();

                // TODO: Is this supposed to be fuzz of previous or next?
                let max_fuzz = (previous_interval as f64 * FUZZ_FACTOR) as u16;
                let fuzz = rng.gen_range(0..=max_fuzz);

                if rng.gen() {
                    next += fuzz;
                } else {
                    next -= fuzz;
                }

                next
            } else {
                // Previous answer was wrong
                std::cmp::max(1, (previous_interval as f64 * WRONG_ANSWER_PENALTY) as u16)
            }
        }
    }
}
