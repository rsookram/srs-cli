//! Utilities for interacting with the user's default text editor.

use anyhow::{bail, Result};
use std::env;
use std::fs;
use std::fs::File;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

/// Opens the default text editor with a file containing the given text. After closing the editor,
/// the contents of the file are returned.
pub fn edit(text: &str) -> Result<String> {
    let path = temp_path()?;

    let temp_file = File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)?;
    let result = get_input(&temp_file, &path, text);

    fs::remove_file(&path)?;

    result
}

/// Returns a path to a file in the OS's temp directory. The file isn't guaranteed to exist
/// already.
fn temp_path() -> Result<PathBuf> {
    let since_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;

    let mut path = env::temp_dir();
    path.push(format!("srs-cli_{}.txt", since_epoch.as_nanos()));

    Ok(path)
}

fn get_input(mut file: &File, path: &Path, text: &str) -> Result<String> {
    write!(file, "{text}")?;

    let cmd = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    if !Command::new(&cmd).arg(&path).status()?.success() {
        bail!("failed to run {cmd}");
    }

    let mut output = String::with_capacity(64);
    file.seek(SeekFrom::Start(0))?;
    file.read_to_string(&mut output)?;

    Ok(output)
}
