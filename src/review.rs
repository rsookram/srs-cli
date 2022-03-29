use anyhow::Result;
use rand::Rng;
use rusqlite::Connection;
use std::path::PathBuf;

const FUZZ_FACTOR: f64 = 0.05; // TODO: consider increasing
const WRONG_ANSWER_PENALTY: f64 = 0.7;

pub fn run(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open(db_path)?;

    todo!()
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
