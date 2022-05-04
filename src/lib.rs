mod schedule;

use anyhow::bail;
use anyhow::Result;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rusqlite::config::DbConfig;
use rusqlite::params;
use rusqlite::types::Null;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use schedule::Schedule;
use std::path::Path;
use time::Duration;
use time::OffsetDateTime;
use time::UtcOffset;

const MIN_INTERVAL_MODIFIER: u16 = 50;
const AUTO_SUSPEND_INTERVAL: u16 = 365;
const WRONG_ANSWERS_FOR_LEECH: u16 = 4;

pub struct Srs {
    conn: Connection,
    offset: UtcOffset,
}

#[derive(Debug, PartialEq)]
pub struct Deck {
    pub id: u64,
    pub name: String,
    pub interval_modifier: u16,
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub struct GlobalStats {
    pub active: u32,
    pub suspended: u32,
    pub leech: u16,
    pub for_review: u16,
}

#[derive(Debug, PartialEq)]
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

        Ok(Self {
            conn,
            offset: UtcOffset::current_local_offset()?,
        })
    }

    #[cfg(test)]
    fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;

        conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;

        Ok(Self {
            conn,
            offset: UtcOffset::UTC,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        self.conn.execute_batch(include_str!("schema.sql"))?;

        Ok(())
    }

    pub fn decks(&self) -> Result<Vec<Deck>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, intervalModifier FROM Deck ORDER BY id")?;

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

        let iter = stmt.query_map([self.start_of_tomorrow()?], |row| {
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

        let mut rng = SmallRng::from_entropy();

        let mut cards_by_deck = vec![];

        let mut current_deck = cards[0].0.clone();
        let mut current_cards = vec![];
        for (deck_name, card) in cards {
            if deck_name != current_deck {
                let old_deck_name = std::mem::take(&mut current_deck);
                let mut cards = std::mem::take(&mut current_cards);

                current_deck = deck_name.clone();

                cards.shuffle(&mut rng);
                cards_by_deck.push((old_deck_name, cards));
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

        let mut schedule = Schedule::new(SmallRng::from_entropy());
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
            [self.end_of_tomorrow()?],
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
        let iter = stmt.query_map([self.thirty_days_ago()?], |row| {
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

    fn start_of_tomorrow(&self) -> Result<u64> {
        let now = OffsetDateTime::now_utc().to_offset(self.offset);
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

    fn end_of_tomorrow(&self) -> Result<u64> {
        let now = OffsetDateTime::now_utc().to_offset(self.offset);
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

    fn thirty_days_ago(&self) -> Result<u64> {
        let now = OffsetDateTime::now_utc().to_offset(self.offset);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn empty_db() -> Result<()> {
        let mut srs = Srs::open_in_memory()?;
        srs.init()?;

        assert_eq!(srs.decks()?.len(), 0);
        assert_eq!(srs.card_previews()?.len(), 0);
        assert_eq!(srs.cards_to_review()?.len(), 0);

        let (global_stats, deck_stats) = srs.stats()?;
        assert_eq!(
            global_stats,
            GlobalStats {
                active: 0,
                suspended: 0,
                leech: 0,
                for_review: 0,
            }
        );
        assert_eq!(deck_stats.len(), 0);

        Ok(())
    }

    #[test]
    fn create_deck() -> Result<()> {
        let mut srs = Srs::open_in_memory()?;
        srs.init()?;

        let deck = create_and_return_deck(&mut srs, "testName")?;
        assert_eq!(deck.name, "testName");
        assert_eq!(deck.interval_modifier, 100);

        assert_eq!(srs.get_deck(deck.id)?, deck);
        assert_eq!(srs.decks()?, vec![deck]);

        Ok(())
    }

    #[test]
    fn edit_deck() -> Result<()> {
        let mut srs = Srs::open_in_memory()?;
        srs.init()?;

        let deck = create_and_return_deck(&mut srs, "testName")?;

        srs.update_interval_modifier(deck.id, 120)?;

        assert_eq!(srs.get_deck(deck.id)?.interval_modifier, 120);

        Ok(())
    }

    #[test]
    fn delete_deck() -> Result<()> {
        let mut srs = Srs::open_in_memory()?;
        srs.init()?;

        let deck = create_and_return_deck(&mut srs, "testName")?;

        srs.delete_deck(deck.id)?;

        assert_eq!(srs.decks()?.len(), 0);

        let (_, deck_stats) = srs.stats()?;
        assert_eq!(deck_stats.len(), 0);

        Ok(())
    }

    #[test]
    fn create_card() -> Result<()> {
        let mut srs = Srs::open_in_memory()?;
        srs.init()?;

        let deck = create_and_return_deck(&mut srs, "testName")?;

        let card = create_and_return_card(&mut srs, &deck, "front", "back")?;

        assert_eq!(card.front, "front");
        assert_eq!(card.back, "back");

        let for_review = srs
            .cards_to_review()?
            .into_iter()
            .next()
            .ok_or(anyhow!("nothing to review"))?;
        assert_eq!(for_review, ("testName".to_string(), vec![card]));

        let (global_stats, deck_stats) = srs.stats()?;
        assert_eq!(
            global_stats,
            GlobalStats {
                active: 1,
                suspended: 0,
                leech: 0,
                for_review: 1,
            }
        );
        assert_eq!(
            deck_stats,
            vec![DeckStats {
                name: "testName".to_string(),
                active: 1,
                suspended: 0,
                leech: 0,
                correct: 0,
                wrong: 0,
            }],
        );

        Ok(())
    }

    #[test]
    fn edit_card() -> Result<()> {
        let mut srs = Srs::open_in_memory()?;
        srs.init()?;

        let deck = create_and_return_deck(&mut srs, "testName")?;

        let card = create_and_return_card(&mut srs, &deck, "front", "back")?;

        srs.update_card(card.id, "new front".to_string(), "new back".to_string())?;

        let edited_card = srs.get_card(card.id)?;
        assert_eq!(edited_card.id, card.id);
        assert_eq!(edited_card.front, "new front");
        assert_eq!(edited_card.back, "new back");

        Ok(())
    }

    #[test]
    fn switch_card() -> Result<()> {
        let mut srs = Srs::open_in_memory()?;
        srs.init()?;

        let deck = create_and_return_deck(&mut srs, "testName")?;
        let deck2 = create_and_return_deck(&mut srs, "another deck")?;

        let card = create_and_return_card(&mut srs, &deck, "front", "back")?;

        srs.switch_deck(card.id, deck2.id)?;

        let for_review = srs
            .cards_to_review()?
            .into_iter()
            .next()
            .ok_or(anyhow!("nothing to review"))?;
        assert_eq!(for_review, ("another deck".to_string(), vec![card]));

        let (_, deck_stats) = srs.stats()?;
        assert_eq!(
            deck_stats,
            vec![
                DeckStats {
                    name: "another deck".to_string(),
                    active: 1,
                    suspended: 0,
                    leech: 0,
                    correct: 0,
                    wrong: 0,
                },
                DeckStats {
                    name: "testName".to_string(),
                    active: 0,
                    suspended: 0,
                    leech: 0,
                    correct: 0,
                    wrong: 0,
                }
            ],
        );

        Ok(())
    }

    #[test]
    fn delete_card() -> Result<()> {
        let mut srs = Srs::open_in_memory()?;
        srs.init()?;

        let deck = create_and_return_deck(&mut srs, "testName")?;

        let card = create_and_return_card(&mut srs, &deck, "front", "back")?;

        srs.delete_card(card.id)?;

        let card = srs.get_card(card.id);

        assert!(card.is_err(), "got a card {:?}", card);

        Ok(())
    }

    fn create_and_return_deck(srs: &mut Srs, name: &str) -> Result<Deck> {
        srs.create_deck(name)?;

        srs.decks()?.pop().ok_or(anyhow!("no decks"))
    }

    fn create_and_return_card(srs: &mut Srs, deck: &Deck, front: &str, back: &str) -> Result<Card> {
        srs.create_card(deck.id, front.to_string(), back.to_string())?;

        let preview = srs
            .card_previews()?
            .into_iter()
            .next()
            .ok_or(anyhow!("no cards"))?;

        srs.get_card(preview.id)
    }
}
