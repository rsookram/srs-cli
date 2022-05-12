use crate::Srs;
use anyhow::anyhow;
use anyhow::Result;
use dialoguer::theme::Theme;
use dialoguer::Confirm;
use srs_cli::Card;
use srs_cli::DeckStats;
use srs_cli::GlobalStats;

pub struct App {
    srs: Srs,
}

impl App {
    pub fn new(srs: Srs) -> Self {
        Self { srs }
    }

    pub fn add(&mut self, deck_id: u64) -> Result<()> {
        let (front, back) = open_editor("", "")?;

        self.srs.create_card(deck_id, front, back)
    }

    pub fn cards(self) -> Result<()> {
        for card in self.srs.card_previews()? {
            let front = card.front.replace('\n', " ");

            if card.is_leech {
                println!("[leech] {} {front}", card.id);
            } else {
                println!("{} {front}", card.id);
            }
        }

        Ok(())
    }

    pub fn create_deck(&mut self, name: &str) -> Result<()> {
        self.srs.create_deck(name)?;

        println!("Created {name}");
        Ok(())
    }

    pub fn decks(&self) -> Result<()> {
        for deck in self.srs.decks()? {
            println!("{} {} - {}%", deck.id, deck.name, deck.interval_modifier);
        }

        Ok(())
    }

    pub fn delete(&mut self, card_id: u64) -> Result<()> {
        let front: String = self.srs.get_card(card_id)?.front;

        if Confirm::new()
            .with_prompt(format!(
                "Are you sure you want to delete '{}'",
                front.replace('\n', " ")
            ))
            .interact()?
        {
            self.srs.delete_card(card_id)?;
            println!("... deleted.");
        }

        Ok(())
    }

    pub fn delete_deck(&mut self, deck_id: u64) -> Result<()> {
        let name = self.srs.get_deck(deck_id)?.name;

        if Confirm::new()
            .with_prompt(format!("Are you sure you want to delete '{name}'"))
            .interact()?
        {
            self.srs.delete_deck(deck_id)?;
            println!("... deleted.");
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

        println!("Set interval modifier for {name} to {modifier}");

        Ok(())
    }

    pub fn review(&mut self) -> Result<()> {
        let cards = self.srs.cards_to_review()?;

        println!(
            "{} cards to review",
            cards.iter().flat_map(|(_, cc)| cc).count()
        );

        for (deck_name, cards) in cards {
            let num_cards = cards.len();

            println!("\n{num_cards} cards to review in {deck_name}\n");

            let mut num_correct = 0;

            for card in cards {
                let is_correct = self.review_card(&card)?;
                if is_correct {
                    num_correct += 1;
                    self.srs.answer_correct(card.id)?;
                } else {
                    self.srs.answer_wrong(card.id)?;
                }

                println!();
            }

            println!("Answered {num_correct}/{num_cards} correctly")
        }

        println!("Finished review");

        Ok(())
    }

    fn review_card(&self, card: &Card) -> Result<bool> {
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

    pub fn stats(&self) -> Result<()> {
        let (global_stats, deck_stats) = self.srs.stats()?;

        self.print_global(&global_stats);

        for stat in deck_stats.iter() {
            println!();
            self.print_deck(stat);
        }

        Ok(())
    }

    fn print_global(&self, stats: &GlobalStats) {
        let total = stats.active + stats.suspended + (stats.leech as u32);
        println!("{} / {total} active", stats.active);

        if stats.leech > 0 {
            println!("{} leeches", stats.leech);
        }

        println!("Review tomorrow: {}", stats.for_review);
    }

    fn print_deck(&self, stats: &DeckStats) {
        let total = stats.active + stats.suspended + (stats.leech as u32);
        println!("{}\n  {} / {total} active", stats.name, stats.active);

        if stats.leech > 0 {
            println!("  {} leeches", stats.leech);
        }

        let num_answered = stats.correct + stats.wrong;

        println!(
            "  Past month accuracy: {:.0}% ({} / {num_answered})",
            if num_answered > 0 {
                stats.correct as f32 / num_answered as f32 * 100.0
            } else {
                100.0
            },
            stats.correct,
        );
    }

    pub fn switch(&mut self, card_id: u64, deck_id: u64) -> Result<()> {
        let front = self.srs.get_card(card_id)?.front;
        let deck_name = self.srs.get_deck(deck_id)?.name;

        if Confirm::new()
            .with_prompt(format!(
                "Are you sure you want to switch '{front}' to {deck_name}?"
            ))
            .interact()?
        {
            self.srs.switch_deck(card_id, deck_id)?;
            println!("... switched.");
        }

        Ok(())
    }
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
