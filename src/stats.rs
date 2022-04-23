use crate::srs::DeckStats;
use crate::srs::GlobalStats;
use crate::srs::Srs;
use anyhow::Result;
use std::fmt;

impl fmt::Display for GlobalStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let total = self.active + self.suspended + (self.leech as u32);
        writeln!(f, "{} / {} active", self.active, total)?;

        if self.leech > 0 {
            writeln!(f, "{} leeches", self.leech)?;
        }

        writeln!(f, "Review tomorrow: {}", self.for_review)
    }
}

impl fmt::Display for DeckStats {
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

pub fn run(srs: Srs) -> Result<()> {
    let (global_stats, deck_stats) = srs.stats()?;

    println!("{global_stats}");

    for stats in deck_stats {
        println!("{stats}");
    }

    Ok(())
}
