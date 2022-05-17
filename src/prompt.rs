//! Prompts displayed to the user to gather input.

use anyhow::bail;
use anyhow::Result;
use std::io::{stdin, stdout, Write};
use termion::clear;
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::IntoRawMode;

/// Displays the given prompt and waits for a yes / no answer. Yes maps to true, and no maps to
/// false.
pub fn binary(prompt: impl AsRef<str>) -> Result<bool> {
    let mut stdout = stdout().into_raw_mode()?;
    write!(stdout, "{} [y/n] ", prompt.as_ref())?;
    stdout.flush()?;

    let stdin = stdin();

    for event in stdin.events() {
        let selection = match event? {
            Event::Key(Key::Char('y')) => true,
            Event::Key(Key::Char('n')) => false,
            Event::Key(Key::Char('q')) | Event::Key(Key::Ctrl('c')) => {
                write!(stdout, "\r\n")?;
                stdout.flush()?;

                bail!("Exiting instead of answering...")
            }
            _ => continue,
        };

        write!(stdout, "{}\r\n", if selection { "yes" } else { "no" })?;
        stdout.flush()?;

        return Ok(selection);
    }

    unreachable!()
}

/// Displays the given prompt and waits until a key is pressed.
pub fn any(prompt: impl AsRef<str>) -> Result<()> {
    let mut stdout = stdout().into_raw_mode()?;
    write!(stdout, "{}", prompt.as_ref())?;
    stdout.flush()?;

    let stdin = stdin();

    for event in stdin.events() {
        match event? {
            Event::Key(Key::Ctrl('c')) => {
                write!(stdout, "\r\n")?;
                stdout.flush()?;

                bail!("Exiting instead of continuing...")
            }
            Event::Key(_) => break,
            _ => continue,
        }
    }

    write!(stdout, "\r{}", clear::AfterCursor)?;
    stdout.flush()?;

    Ok(())
}
