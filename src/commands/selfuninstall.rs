// src/commands/selfuninstall.rs
//
// `dsi selfuninstall` — Remove the dsi binary itself, leaving SDKs intact.
//
// Also rewrites the shell profile blocks added by install.sh: dsi-specific
// lines (the dsi bin dir on PATH, completion hooks) are removed, while the
// .NET SDK paths (DOTNET_ROOT and ~/.dotnet on PATH) are kept so installed
// SDKs stay usable. If no SDKs are installed, the whole block is removed.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result as AnyResult, bail};
use clap::Parser;

use crate::paths::DsiPaths;
use crate::prompt::confirm;

// Must match the markers written by install.sh.
const MARKER_BEGIN: &str = "# >>> dsi >>>";
const MARKER_END: &str = "# <<< dsi <<<";

#[derive(Parser)]
pub struct SelfUninstallArgs {
    /// Skip the confirmation prompt
    #[arg(short, long)]
    pub yes: bool,
}

enum ProfileStyle {
    Posix,
    Fish,
    Powershell,
    Nushell,
}

impl ProfileStyle {
    /// The block lines to keep: only the .NET SDK paths, mirroring what
    /// install.sh writes minus the dsi bin dir and completion hook.
    fn dotnet_lines(&self, root: &str) -> Vec<String> {
        match self {
            ProfileStyle::Posix => vec![
                format!("export DOTNET_ROOT=\"{root}\""),
                format!("export PATH=\"{root}:{root}/tools:$PATH\""),
            ],
            ProfileStyle::Fish => vec![
                format!("set -gx DOTNET_ROOT \"{root}\""),
                format!("fish_add_path \"{root}\" \"{root}/tools\""),
            ],
            ProfileStyle::Powershell => vec![
                format!("$env:DOTNET_ROOT = \"{root}\""),
                format!("$env:PATH = \"{root}:{root}/tools:$env:PATH\""),
            ],
            ProfileStyle::Nushell => vec![
                format!("$env.DOTNET_ROOT = \"{root}\""),
                format!("$env.PATH = ($env.PATH | prepend [\"{root}\" \"{root}/tools\"])"),
            ],
        }
    }
}

/// The profile files install.sh may have configured.
fn profile_files(home: &Path) -> Vec<(PathBuf, ProfileStyle)> {
    let zdotdir = std::env::var_os("ZDOTDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.to_path_buf());

    vec![
        (home.join(".bashrc"), ProfileStyle::Posix),
        (zdotdir.join(".zshrc"), ProfileStyle::Posix),
        (home.join(".config/fish/config.fish"), ProfileStyle::Fish),
        (
            home.join(".config/powershell/Microsoft.PowerShell_profile.ps1"),
            ProfileStyle::Powershell,
        ),
        (home.join(".config/nushell/config.nu"), ProfileStyle::Nushell),
    ]
}

/// Replace the dsi block in `file` with `replacement` lines (markers kept),
/// or strip the block entirely when `replacement` is empty.
/// Returns true if the file was rewritten.
fn clean_profile(file: &Path, replacement: &[String]) -> AnyResult<bool> {
    let content = match std::fs::read_to_string(file) {
        Ok(content) => content,
        Err(_) => return Ok(false), // missing or unreadable — nothing to do
    };

    if !content.lines().any(|line| line.trim() == MARKER_BEGIN) {
        return Ok(false);
    }

    let mut out: Vec<&str> = Vec::new();
    let mut in_block = false;
    for line in content.lines() {
        if line.trim() == MARKER_BEGIN {
            in_block = true;
            if !replacement.is_empty() {
                out.push(MARKER_BEGIN);
                out.extend(replacement.iter().map(String::as_str));
                out.push(MARKER_END);
            }
            continue;
        }
        if line.trim() == MARKER_END {
            in_block = false;
            continue;
        }
        if !in_block {
            out.push(line);
        }
    }

    let mut new_content = out.join("\n");
    new_content.push('\n');
    if new_content == content {
        return Ok(false);
    }

    std::fs::write(file, new_content)
        .with_context(|| format!("Failed to update {}", file.display()))?;
    Ok(true)
}

pub async fn run(args: SelfUninstallArgs) -> AnyResult<()> {
    let exe = std::env::current_exe().context("Could not determine the dsi binary location")?;
    let paths = DsiPaths::resolve()?;
    let keep_sdks = paths.dotnet_root.exists();

    println!("This will remove dsi from {}.", exe.display());
    if keep_sdks {
        println!(
            "SDKs in {} will NOT be removed; shell profiles keep the .NET SDK paths.",
            paths.dotnet_root.display()
        );
    } else {
        println!("No SDKs found; dsi entries will be removed from shell profiles.");
    }

    if !args.yes {
        let proceed = confirm("Continue?", false)?;
        if !proceed {
            println!("Aborted.");
            return Ok(());
        }
    }

    std::fs::remove_file(&exe).with_context(|| format!("Failed to remove {}", exe.display()))?;

    // Sanity check: removal can silently fail on some filesystems.
    if exe.exists() {
        bail!(
            "dsi binary still present at {} after removal",
            exe.display()
        );
    }

    // Undo the installer's shell profile changes, keeping SDK paths if any
    // SDKs remain installed.
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let root = paths.dotnet_root.display().to_string();
    for (file, style) in profile_files(&home) {
        let replacement = if keep_sdks {
            style.dotnet_lines(&root)
        } else {
            Vec::new()
        };
        match clean_profile(&file, &replacement) {
            Ok(true) => println!("✓ cleaned {}", file.display()),
            Ok(false) => {}
            Err(e) => eprintln!("warning: {}", e),
        }
    }

    println!("✓ Removed dsi");
    Ok(())
}
