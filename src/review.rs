use anyhow::Result;
use chrono::Duration;
use chrono::Utc;
use rand::Rng;
use rusqlite::params;
use rusqlite::types::Null;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use std::path::PathBuf;

const AUTO_SUSPEND_INTERVAL: u16 = 365;
const WRONG_ANSWERS_FOR_LEECH: u16 = 4;
const FUZZ_FACTOR: f64 = 0.05; // TODO: consider increasing
const WRONG_ANSWER_PENALTY: f64 = 0.7;

pub fn run(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open(db_path)?;

    todo!()
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
