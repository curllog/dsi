// src/prompt.rs
//
// Small interactive helpers shared by destructive commands.

use std::io::{self, Write};

use anyhow::{Context, Result as AnyResult};

/// Ask a yes/no question on stdin.
///
/// `default_yes` controls what a bare Enter means and which letter is
/// capitalized in the `[y/N]` / `[Y/n]` hint.
pub fn confirm(question: &str, default_yes: bool) -> AnyResult<bool> {
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
    print!("{} {} ", question, hint);
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read confirmation from stdin")?;

    let answer = input.trim().to_lowercase();
    Ok(match answer.as_str() {
        "" => default_yes,
        "y" | "yes" => true,
        _ => false,
    })
}
