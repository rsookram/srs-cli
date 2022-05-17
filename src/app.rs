use crate::Srs;
use anyhow::anyhow;
use anyhow::Result;
use srs_cli::prompt;
use srs_cli::Card;
use srs_cli::DeckStats;
use srs_cli::GlobalStats;
use std::io;

pub struct App<W: io::Write> {
    srs: Srs,
    output: W,
}

impl<W: io::Write> App<W> {
    pub fn new(srs: Srs, output: W) -> Self {
        Self { srs, output }
    }

    pub fn add(&mut self, deck_id: u64) -> Result<()> {
        let (front, back) = open_editor("", "")?;

        self.srs.create_card(deck_id, front, back)
    }

    pub fn cards(&mut self) -> Result<()> {
        for card in self.srs.card_previews()? {
            let front = card.front.replace('\n', " ");

            if card.is_leech {
                writeln!(self.output, "[leech] {} {front}", card.id)?;
            } else {
                writeln!(self.output, "{} {front}", card.id)?;
            }
        }

        Ok(())
    }

    pub fn create_deck(&mut self, name: &str) -> Result<()> {
        self.srs.create_deck(name)?;

        writeln!(self.output, "Created {name}")?;
        Ok(())
    }

    pub fn decks(&mut self) -> Result<()> {
        for deck in self.srs.decks()? {
            writeln!(
                self.output,
                "{} {} - {}%",
                deck.id, deck.name, deck.interval_modifier
            )?;
        }

        Ok(())
    }

    pub fn delete(&mut self, card_id: u64) -> Result<()> {
        let front: String = self.srs.get_card(card_id)?.front;

        if prompt::binary(format!(
            "Are you sure you want to delete '{}'?",
            front.replace('\n', " ")
        ))? {
            self.srs.delete_card(card_id)?;
            writeln!(self.output, "... deleted.")?;
        }

        Ok(())
    }

    pub fn delete_deck(&mut self, deck_id: u64) -> Result<()> {
        let name = self.srs.get_deck(deck_id)?.name;

        if prompt::binary(format!("Are you sure you want to delete '{name}'?"))? {
            self.srs.delete_deck(deck_id)?;
            writeln!(self.output, "... deleted.")?;
        }

        Ok(())
    }

    pub fn edit(&mut self, card_id: u64) -> Result<()> {
        let card = self.srs.get_card(card_id)?;

        let (front, back) = open_editor(&card.front, &card.back)?;

        self.srs.update_card(card_id, front, back)
    }

    pub fn init(&mut self) -> Result<()> {
        self.srs.init()
    }

    pub fn int_mod(&mut self, deck_id: u64, modifier: u16) -> Result<()> {
        let name: String = self.srs.get_deck(deck_id)?.name;

        self.srs.update_interval_modifier(deck_id, modifier)?;

        writeln!(
            self.output,
            "Set interval modifier for {name} to {modifier}"
        )?;

        Ok(())
    }

    pub fn review(&mut self) -> Result<()> {
        let cards = self.srs.cards_to_review()?;

        writeln!(
            self.output,
            "{} cards to review",
            cards.iter().flat_map(|(_, cc)| cc).count()
        )?;

        for (deck_name, cards) in cards {
            let num_cards = cards.len();

            writeln!(
                self.output,
                "\n{num_cards} cards to review in {deck_name}\n"
            )?;

            let mut num_correct = 0;

            for card in cards {
                let is_correct = self.review_card(&card)?;
                if is_correct {
                    num_correct += 1;
                    self.srs.answer_correct(card.id)?;
                } else {
                    self.srs.answer_wrong(card.id)?;
                }

                writeln!(self.output)?;
            }

            writeln!(self.output, "Answered {num_correct}/{num_cards} correctly")?;
        }

        writeln!(self.output, "Finished review")?;

        Ok(())
    }

    fn review_card(&mut self, card: &Card) -> Result<bool> {
        writeln!(self.output, "{}\n", &card.front)?;

        prompt::any("Press any key to show answer")?;

        writeln!(self.output, "{}", "─".repeat(39))?;

        writeln!(self.output, "{}\n", &card.back)?;

        prompt::binary("Correct?")
    }

    pub fn stats(&mut self) -> Result<()> {
        let (global_stats, deck_stats) = self.srs.stats()?;

        self.output_global(&global_stats)?;

        for stat in deck_stats.iter() {
            writeln!(self.output)?;
            self.output_deck(stat)?;
        }

        Ok(())
    }

    fn output_global(&mut self, stats: &GlobalStats) -> Result<()> {
        let total = stats.active + stats.suspended + (stats.leech as u32);
        writeln!(self.output, "{} / {total} active", stats.active)?;

        if stats.leech > 0 {
            writeln!(self.output, "{} leeches", stats.leech)?;
        }

        writeln!(self.output, "Review tomorrow: {}", stats.for_review)?;

        Ok(())
    }

    fn output_deck(&mut self, stats: &DeckStats) -> Result<()> {
        let total = stats.active + stats.suspended + (stats.leech as u32);
        writeln!(
            self.output,
            "{}\n  {} / {total} active",
            stats.name, stats.active
        )?;

        if stats.leech > 0 {
            writeln!(self.output, "  {} leeches", stats.leech)?;
        }

        let num_answered = stats.correct + stats.wrong;

        writeln!(
            self.output,
            "  Past month accuracy: {:.0}% ({} / {num_answered})",
            if num_answered > 0 {
                stats.correct as f32 / num_answered as f32 * 100.0
            } else {
                100.0
            },
            stats.correct,
        )?;

        Ok(())
    }

    pub fn switch(&mut self, card_id: u64, deck_id: u64) -> Result<()> {
        let front = self.srs.get_card(card_id)?.front;
        let deck_name = self.srs.get_deck(deck_id)?.name;

        if prompt::binary(format!(
            "Are you sure you want to switch '{front}' to {deck_name}?"
        ))? {
            self.srs.switch_deck(card_id, deck_id)?;
            writeln!(self.output, "... switched.")?;
        }

        Ok(())
    }
}

fn open_editor(front: &str, back: &str) -> Result<(String, String)> {
    let divider = "----------";
    let template = format!("{front}\n{divider}\n{back}\n");

    let output = scrawl::with(&template)?;

    output
        .split_once(divider)
        .map(|(front, back)| (front.trim().to_string(), back.trim().to_string()))
        .ok_or_else(|| anyhow!("Missing divider between front and back of card"))
        .and_then(|(front, back)| {
            if front.is_empty() {
                Err(anyhow!("Front of card can't be empty"))
            } else {
                Ok((front, back))
            }
        })
}
