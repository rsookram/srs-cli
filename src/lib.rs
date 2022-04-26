use anyhow::bail;
use anyhow::Result;
use rand::seq::SliceRandom;
use rand::Rng;
use rusqlite::config::DbConfig;
use rusqlite::params;
use rusqlite::types::Null;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use std::path::Path;
use time::Duration;
use time::OffsetDateTime;

const MIN_INTERVAL_MODIFIER: u16 = 50;
const AUTO_SUSPEND_INTERVAL: u16 = 365;
const WRONG_ANSWERS_FOR_LEECH: u16 = 4;
const WRONG_ANSWER_PENALTY: f64 = 0.7;

pub struct Srs {
    conn: Connection,
}

#[derive(Debug)]
pub struct Deck {
    pub id: u64,
    pub name: String,
    pub interval_modifier: u16,
}

#[derive(Debug)]
pub struct Card {
    pub id: u64,
    pub front: String,
    pub back: String,
}

#[derive(Debug)]
pub struct CardPreview {
    pub id: u64,
    pub front: String,
    pub is_leech: bool,
}

#[derive(Debug)]
pub struct GlobalStats {
    pub active: u32,
    pub suspended: u32,
    pub leech: u16,
    pub for_review: u16,
}

#[derive(Debug)]
pub struct DeckStats {
    pub name: String,
    pub active: u32,
    pub suspended: u32,
    pub leech: u16,
    pub correct: u16,
    pub wrong: u16,
}

impl Srs {
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;

        Ok(Self { conn })
    }

    pub fn decks(&self) -> Result<Vec<Deck>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, intervalModifier FROM Deck")?;

        let iter = stmt.query_map([], |row| {
            Ok(Deck {
                id: row.get(0)?,
                name: row.get(1)?,
                interval_modifier: row.get(2)?,
            })
        })?;

        let r: Result<_, rusqlite::Error> = iter.collect();

        Ok(r?)
    }

    pub fn get_deck(&self, id: u64) -> Result<Deck> {
        Ok(self.conn.query_row(
            "SELECT id, name, intervalModifier FROM Deck WHERE id = ?",
            [id],
            |row| {
                Ok(Deck {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    interval_modifier: row.get(2)?,
                })
            },
        )?)
    }

    pub fn create_deck(&mut self, name: &str) -> Result<()> {
        if name.is_empty() {
            bail!("deck name can't be empty");
        }

        let now: u64 = (OffsetDateTime::now_utc().unix_timestamp() * 1000)
            .try_into()
            .expect("valid timestamp");

        self.conn.execute(
            "INSERT INTO Deck(name, creationTimestamp) VALUES (?, ?)",
            params![name, now],
        )?;

        Ok(())
    }

    pub fn delete_deck(&mut self, id: u64) -> Result<()> {
        self.conn.execute("DELETE FROM Deck WHERE id = ?", [id])?;

        Ok(())
    }

    pub fn update_interval_modifier(&mut self, id: u64, modifier: u16) -> Result<()> {
        if modifier < MIN_INTERVAL_MODIFIER {
            bail!(format!(
                "must be > {MIN_INTERVAL_MODIFIER}, given {modifier}"
            ));
        }

        self.conn.execute(
            "
            UPDATE Deck
            SET intervalModifier = ?
            WHERE id = ?
            ",
            params![modifier, id],
        )?;

        Ok(())
    }

    pub fn get_card(&self, id: u64) -> Result<Card> {
        Ok(self.conn.query_row(
            "
            SELECT id, front, back
            FROM Card
            WHERE id = ?
            ",
            [id],
            |row| {
                Ok(Card {
                    id: row.get(0)?,
                    front: row.get(1)?,
                    back: row.get(2)?,
                })
            },
        )?)
    }

    pub fn create_card(&mut self, deck_id: u64, front: String, back: String) -> Result<()> {
        let tx = self.conn.transaction()?;

        let now: u64 = (OffsetDateTime::now_utc().unix_timestamp() * 1000)
            .try_into()
            .expect("valid timestamp");

        let card_id: u64 = tx.query_row(
            "INSERT INTO Card(deckId, front, back, creationTimestamp) VALUES (?, ?, ?, ?) RETURNING *",
            params![deck_id, front, back, now],
            |row| row.get(0),
        )?;

        tx.execute(
            "INSERT INTO Schedule(cardId, scheduledForTimestamp, intervalDays) VALUES (?, ?, ?)",
            params![card_id, now, 0],
        )?;

        tx.commit()?;

        Ok(())
    }

    pub fn delete_card(&mut self, id: u64) -> Result<()> {
        self.conn.execute("DELETE FROM Card WHERE id = ?", [id])?;

        Ok(())
    }

    pub fn update_card(&mut self, id: u64, front: String, back: String) -> Result<()> {
        self.conn.execute(
            "UPDATE Card SET front=?, back=? WHERE id=?",
            params![front, back, id],
        )?;

        Ok(())
    }

    pub fn switch_deck(&mut self, card_id: u64, deck_id: u64) -> Result<()> {
        self.conn.execute(
            "UPDATE Card SET deckId = ? WHERE id = ?",
            [deck_id, card_id],
        )?;

        Ok(())
    }

    pub fn card_previews(&self) -> Result<Vec<CardPreview>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, front, isLeech
            FROM Card
            JOIN Schedule ON Card.id = Schedule.cardId
            ORDER BY isLeech DESC, creationTimestamp DESC;
            ",
        )?;

        let iter = stmt.query_map([], |row| {
            Ok(CardPreview {
                id: row.get(0)?,
                front: row.get(1)?,
                is_leech: row.get(2)?,
            })
        })?;

        let r: Result<_, rusqlite::Error> = iter.collect();

        Ok(r?)
    }

    pub fn cards_to_review(&self) -> Result<Vec<(String, Vec<Card>)>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT Deck.name, Card.id, Card.front, Card.back
            FROM Card JOIN Schedule ON Card.id = Schedule.cardId JOIN Deck ON Card.deckId = Deck.id
            WHERE isLeech = 0 AND scheduledForTimestamp < ?
            ORDER BY Card.deckId, scheduledForTimestamp
            ",
        )?;

        let iter = stmt.query_map([start_of_tomorrow()?], |row| {
            Ok((
                row.get(0)?,
                Card {
                    id: row.get(1)?,
                    front: row.get(2)?,
                    back: row.get(3)?,
                },
            ))
        })?;

        let r: Result<_, rusqlite::Error> = iter.collect();

        let cards: Vec<(String, Card)> = r?;

        if cards.is_empty() {
            return Ok(vec![]);
        }

        let mut rng = rand::thread_rng();

        let mut cards_by_deck = vec![];

        let mut current_deck = cards[0].0.clone();
        let mut current_cards = vec![];
        for (deck_name, card) in cards {
            if deck_name != current_deck {
                let deck_name = std::mem::take(&mut current_deck);
                let mut cards = std::mem::take(&mut current_cards);

                current_deck = deck_name.clone();

                cards.shuffle(&mut rng);
                cards_by_deck.push((deck_name, cards));
            }

            current_cards.push(card);
        }

        current_cards.shuffle(&mut rng);
        cards_by_deck.push((current_deck, current_cards));

        Ok(cards_by_deck)
    }

    pub fn answer_correct(&mut self, card_id: u64) -> Result<()> {
        let now_date_time = OffsetDateTime::now_utc();

        let now: u64 = (now_date_time.unix_timestamp() * 1000)
            .try_into()
            .expect("valid timestamp");

        let tx = self.conn.transaction()?;

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

        let mut schedule = Schedule::new(rand::thread_rng());
        let num_days = schedule.next_interval(interval_days, was_correct, interval_modifier);

        if num_days >= AUTO_SUSPEND_INTERVAL {
            tx.execute(
                "UPDATE Schedule SET scheduledForTimestamp = ?, intervalDays = ? WHERE cardId = ?",
                params![Null, Null, card_id],
            )?;
        } else {
            let num_days_ms: u64 = Duration::days(num_days.into())
                .whole_milliseconds()
                .try_into()
                .expect("valid duration");

            tx.execute(
                "UPDATE Schedule SET scheduledForTimestamp = ?, intervalDays = ? WHERE cardId = ?",
                params![now + num_days_ms, num_days, card_id],
            )?;
        }

        tx.commit()?;

        Ok(())
    }

    pub fn answer_wrong(&mut self, card_id: u64) -> Result<()> {
        let now: u64 = (OffsetDateTime::now_utc().unix_timestamp() * 1000)
            .try_into()
            .expect("valid timestamp");

        let tx = self.conn.transaction()?;

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

    pub fn stats(&self) -> Result<(GlobalStats, Vec<DeckStats>)> {
        let global_stats = self.conn.query_row(
            "
            SELECT
                (SELECT COUNT(*)
                FROM Card JOIN Schedule ON Card.id = Schedule.cardId
                WHERE scheduledForTimestamp IS NOT NULL AND isLeech = 0) AS active,

                (SELECT COUNT(*)
                FROM Card JOIN Schedule ON Card.id = Schedule.cardId
                WHERE scheduledForTimestamp IS NULL AND isLeech = 0) AS suspended,

                (SELECT COUNT(*)
                FROM Card JOIN Schedule ON Card.id = Schedule.cardId
                WHERE isLeech = 1) AS leech,

                (SELECT COUNT(*)
                FROM Schedule
                WHERE scheduledForTimestamp < :reviewSpanEnd) AS forReview
            ",
            [end_of_tomorrow()?],
            |row| {
                Ok(GlobalStats {
                    active: row.get(0)?,
                    suspended: row.get(1)?,
                    leech: row.get(2)?,
                    for_review: row.get(3)?,
                })
            },
        )?;

        let mut stmt = self.conn.prepare(
            "
            SELECT
                name,

                (SELECT COUNT(*)
                FROM Card JOIN Schedule ON Card.id = Schedule.cardId
                WHERE Card.deckId = d.id AND scheduledForTimestamp IS NOT NULL AND isLeech = 0) AS active,

                (SELECT COUNT(*)
                FROM Card JOIN Schedule ON Card.id = Schedule.cardId
                WHERE Card.deckId = d.id AND scheduledForTimestamp IS NULL AND isLeech = 0) AS suspended,

                (SELECT COUNT(*)
                FROM Card JOIN Schedule ON Card.id = Schedule.cardId
                WHERE Card.deckId = d.id AND isLeech = 1) AS leech,

                (SELECT COUNT(*)
                FROM Card JOIN Answer ON Card.id = Answer.cardId
                WHERE Card.deckId = d.id AND isCorrect = 1 AND Answer.timestamp > :accuracySinceTimestamp) AS correct,

                (SELECT COUNT(*)
                FROM Card JOIN Answer ON Card.id = Answer.cardId
                WHERE Card.deckId = d.id AND isCorrect = 0 AND Answer.timestamp > :accuracySinceTimestamp) AS wrong
            FROM Deck AS d
            ORDER BY name
            "
        )?;
        let iter = stmt.query_map([thirty_days_ago()?], |row| {
            Ok(DeckStats {
                name: row.get(0)?,
                active: row.get(1)?,
                suspended: row.get(2)?,
                leech: row.get(3)?,
                correct: row.get(4)?,
                wrong: row.get(5)?,
            })
        })?;

        let deck_stats: Result<_, rusqlite::Error> = iter.collect();

        Ok((global_stats, deck_stats?))
    }
}

fn start_of_tomorrow() -> Result<u64> {
    let now = OffsetDateTime::now_local()?;
    let start_of_tomorrow = now
        .date()
        .saturating_add(Duration::days(1))
        .with_hms(0, 0, 0)
        .expect("valid time")
        .assume_offset(now.offset());

    Ok((start_of_tomorrow.unix_timestamp() * 1000)
        .try_into()
        .expect("valid timestamp"))
}

fn end_of_tomorrow() -> Result<u64> {
    let now = OffsetDateTime::now_local()?;
    let end_of_tomorrow = now
        .date()
        .saturating_add(Duration::days(1))
        .with_hms(23, 59, 59)
        .expect("valid time")
        .assume_offset(now.offset());

    Ok((end_of_tomorrow.unix_timestamp() * 1000)
        .try_into()
        .expect("valid timestamp"))
}

fn thirty_days_ago() -> Result<u64> {
    let now = OffsetDateTime::now_local()?;
    let end_of_tomorrow = now
        .date()
        .saturating_sub(Duration::days(30))
        .with_hms(0, 0, 0)
        .expect("valid time")
        .assume_offset(now.offset());

    Ok((end_of_tomorrow.unix_timestamp() * 1000)
        .try_into()
        .expect("valid timestamp"))
}

struct Schedule<R: Rng> {
    rng: R,
}

impl<R: Rng> Schedule<R> {
    fn new(rng: R) -> Self {
        Schedule { rng }
    }

    fn next_interval(
        &mut self,
        previous_interval: u16,
        was_correct: Option<bool>,
        interval_modifier: u16,
    ) -> u16 {
        match previous_interval {
            // Newly added card answered correctly
            0 => 1,
            // First review. The wrong answer penalty isn't applied since it's rare to answer
            // incorrectly on the first review.
            1 => 4,
            _ => {
                let was_correct = was_correct.expect("previously answered the card");

                if was_correct {
                    // Previous answer was correct
                    let mut next =
                        (previous_interval as f64 * 2.5 * (interval_modifier as f64 / 100.0))
                            as u16;

                    let max_fuzz =
                        (previous_interval as f64 * self.fuzz_factor(previous_interval)) as u16;
                    let fuzz = self.rng.gen_range(0..=max_fuzz);

                    if self.rng.gen() {
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

    fn fuzz_factor(&self, previous_interval: u16) -> f64 {
        if previous_interval < 7 {
            0.25
        } else if previous_interval < 30 {
            0.15
        } else {
            0.05
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    struct NotRandom;

    impl RngCore for NotRandom {
        fn next_u32(&mut self) -> u32 {
            self.next_u64() as u32
        }

        fn next_u64(&mut self) -> u64 {
            0
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            dest.fill(0);
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
            Ok(self.fill_bytes(dest))
        }
    }

    #[test]
    fn first_answer() {
        let mut schedule = Schedule::new(NotRandom);

        let next = schedule.next_interval(0, None, 100);

        assert_eq!(next, 1);
    }

    #[test]
    fn first_review() {
        let mut schedule = Schedule::new(NotRandom);

        let next = schedule.next_interval(1, Some(true), 100);

        assert_eq!(next, 4);
    }

    #[test]
    fn apply_wrong_penalty() {
        let mut schedule = Schedule::new(NotRandom);

        let next = schedule.next_interval(50, Some(false), 100);

        assert_eq!(next, (50 as f64 * WRONG_ANSWER_PENALTY) as u16);
    }

    #[test]
    fn correct_answer() {
        let mut schedule = Schedule::new(NotRandom);

        let next = schedule.next_interval(50, Some(true), 100);

        assert_eq!(next, 125);
    }

    #[test]
    fn increase_by_interval_modifier() {
        let mut schedule = Schedule::new(NotRandom);

        let next = schedule.next_interval(50, Some(true), 200);

        assert_eq!(next, 250);
    }

    #[test]
    #[should_panic]
    fn expect_previous_answer_for_large_interval() {
        let mut schedule = Schedule::new(NotRandom);

        schedule.next_interval(50, None, 200);
    }
}
