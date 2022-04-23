use crate::srs::Card;
use crate::srs::Srs;
use anyhow::Result;
use dialoguer::theme::Theme;
use dialoguer::Confirm;

pub fn run(mut srs: Srs) -> Result<()> {
    let cards = srs.cards_to_review()?;

    for (deck_name, cards) in cards {
        let num_cards = cards.len();

        println!("{num_cards} cards to review in {deck_name}\n");

        let mut num_correct = 0;

        for card in cards {
            let is_correct = review_card(&card)?;
            if is_correct {
                num_correct += 1;
                srs.answer_correct(card.id)?;
            } else {
                srs.answer_wrong(card.id)?;
            }

            println!();
        }

        println!("Answered {num_correct}/{num_cards} correctly")
    }

    println!("Finished review");

    Ok(())
}

fn review_card(card: &Card) -> Result<bool> {
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
