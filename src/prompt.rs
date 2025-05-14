//! Prompts displayed to the user to gather input.

use crate::error::Result;
use std::io::{stdin, stdout, BufRead, Write};

/// Displays the given prompt and waits for a yes / no answer. Yes maps to true, and no maps to
/// false.
pub fn binary(prompt: impl AsRef<str>) -> Result<bool> {
    let mut stdout = stdout();
    write!(stdout, "{} [y/n] ", prompt.as_ref())?;
    stdout.flush()?;

    let stdin = stdin().lock();

    for line in stdin.lines() {
        let selection = match line?.as_str() {
            "y" => true,
            "n" => false,
            "q" => {
                return Err("Exiting instead of answering...".into());
            }
            _ => continue,
        };

        return Ok(selection);
    }

    Err("No more input. Exiting instead of answering...".into())
}

/// Displays the given prompt and waits until enter is pressed.
pub fn enter(prompt: impl AsRef<str>) -> Result<()> {
    let mut stdout = stdout();
    write!(stdout, "{}", prompt.as_ref())?;
    stdout.flush()?;

    let stdin = stdin();
    stdin.lock().skip_until(b'\n')?;

    write!(stdout, "\x1B[F\r\x1BJ")?;

    Ok(())
}
