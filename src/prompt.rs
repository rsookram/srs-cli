//! Prompts displayed to the user to gather input.

use anyhow::Result;
use dialoguer::theme::Theme;
use dialoguer::Confirm;

/// Displays the given prompt and waits for a yes / no answer. Yes maps to true, and no maps to
/// false.
pub fn binary(prompt: impl AsRef<str>) -> Result<bool> {
    Ok(Confirm::new().with_prompt(prompt.as_ref()).interact()?)
}

/// Displays the given prompt and waits until a key is pressed.
pub fn any(prompt: impl AsRef<str>) -> Result<()> {
    Confirm::with_theme(&PlainPrompt)
        .with_prompt(prompt.as_ref())
        .default(true)
        .show_default(false)
        .report(false)
        .interact()?;

    Ok(())
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
