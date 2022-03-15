use anyhow::Result;
use chrono::Duration;
use chrono::Local;
use rusqlite::Connection;
use rusqlite::OpenFlags;
use std::fmt;
use std::path::PathBuf;

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

    let tomorrow = Local::today().succ();
    let tomorrow_end = tomorrow.and_hms(23, 59, 59).timestamp_millis();

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
        [tomorrow_end],
        |row| {
            Ok(Global {
                active: row.get(0)?,
                suspended: row.get(1)?,
                leech: row.get(2)?,
                for_review: row.get(3)?,
            })
        },
    )?;

    let thirty_days_ago = (Local::today() - Duration::days(30))
        .and_hms(0, 0, 0)
        .timestamp_millis();

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
    let deck_stats_iter = stmt.query_map([thirty_days_ago], |row| {
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
