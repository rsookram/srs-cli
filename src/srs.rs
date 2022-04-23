use anyhow::bail;
use anyhow::Result;
use rusqlite::config::DbConfig;
use rusqlite::params;
use rusqlite::Connection;
use std::path::PathBuf;
use time::Duration;
use time::OffsetDateTime;

const MIN_INTERVAL_MODIFIER: u16 = 50;

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
pub struct CardPreview {
    pub id: u64,
    pub front: String,
    pub is_leech: bool,
}

#[derive(Debug)]
pub struct Card {
    pub id: u64,
    pub front: String,
    pub back: String,
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
    pub fn open(db_path: &PathBuf) -> Result<Self> {
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
