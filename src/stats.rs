use anyhow::Result;
use rusqlite::Connection;
use rusqlite::OpenFlags;
use std::fmt;
use std::path::PathBuf;
use time::Duration;
use time::OffsetDateTime;

#[derive(Debug)]
struct Global {
    active: u32,
    suspended: u32,
    leech: u16,
    for_review: u16,
}

impl fmt::Display for Global {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let total = self.active + self.suspended + (self.leech as u32);
        writeln!(f, "{} / {} active", self.active, total)?;

        if self.leech > 0 {
            writeln!(f, "{} leeches", self.leech)?;
        }

        writeln!(f, "Review tomorrow: {}", self.for_review)
    }
}

#[derive(Debug)]
struct Deck {
    name: String,
    active: u32,
    suspended: u32,
    leech: u16,
    correct: u16,
    wrong: u16,
}

impl fmt::Display for Deck {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let total = self.active + self.suspended + (self.leech as u32);
        writeln!(f, "{}\n{} / {} active", self.name, self.active, total)?;

        if self.leech > 0 {
            writeln!(f, "{} leeches", self.leech)?;
        }

        let num_answered = self.correct + self.wrong;

        writeln!(
            f,
            "Past month accuracy: {:.0}% ({} / {})",
            if num_answered > 0 {
                self.correct as f32 / num_answered as f32 * 100.0
            } else {
                100.0
            },
            self.correct,
            num_answered,
        )
    }
}

pub fn run(db_path: &PathBuf) -> Result<()> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let global_stats = conn.query_row(
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
            Ok(Global {
                active: row.get(0)?,
                suspended: row.get(1)?,
                leech: row.get(2)?,
                for_review: row.get(3)?,
            })
        },
    )?;

    let mut stmt = conn.prepare(
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
                ORDER BY name;
                "
            )?;
    let deck_stats_iter = stmt.query_map([thirty_days_ago()?], |row| {
        Ok(Deck {
            name: row.get(0)?,
            active: row.get(1)?,
            suspended: row.get(2)?,
            leech: row.get(3)?,
            correct: row.get(4)?,
            wrong: row.get(5)?,
        })
    })?;

    println!("{global_stats}");

    for row in deck_stats_iter {
        let deck_stats = row?;

        println!("{deck_stats}");
    }
    Ok(())
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
