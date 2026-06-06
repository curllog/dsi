// src/commands/prune.rs
//
// `dsi prune` — Remove outdated patches, keeping only the latest per band.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result as AnyResult};
use clap::Parser;

use crate::paths::DsiPaths;
use crate::prompt::confirm;
use crate::version::{feature_band, version_key};

#[derive(Parser)]
pub struct PruneArgs {
    /// Show what would be removed without deleting anything
    #[arg(long)]
    pub dry_run: bool,
    /// Skip the confirmation prompt
    #[arg(short, long)]
    pub yes: bool,
}

pub async fn run(args: PruneArgs) -> AnyResult<()> {
    let paths = DsiPaths::resolve()?;
    let installed = paths.installed_sdks()?;

    if installed.is_empty() {
        println!();
        println!("  No .NET SDKs installed — nothing to prune.");
        println!();
        return Ok(());
    }

    // Group versions by feature band, keeping the latest patch per band.
    let mut bands: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for version in &installed {
        bands
            .entry(feature_band(version))
            .or_default()
            .push(version.clone());
    }

    let mut to_keep: Vec<String> = Vec::new();
    let mut to_remove: Vec<String> = Vec::new();
    for versions in bands.values() {
        let latest = versions
            .iter()
            .max_by(|a, b| version_key(a).cmp(&version_key(b)))
            .expect("band always has at least one version");
        for v in versions {
            if v == latest {
                to_keep.push(v.clone());
            } else {
                to_remove.push(v.clone());
            }
        }
    }

    to_keep.sort_by_key(|a| version_key(a));
    to_remove.sort_by_key(|a| version_key(a));

    if to_remove.is_empty() {
        println!();
        println!("Nothing to prune — every feature band already has only its latest patch.");
        println!();
        return Ok(());
    }

    let verb = if args.dry_run {
        "Would remove"
    } else {
        "Will remove"
    };
    println!();
    println!("{}:", verb);
    for v in &to_remove {
        println!("  {}  (superseded in band {})", v, feature_band(v));
    }
    println!();
    println!("Would keep:");
    for v in &to_keep {
        println!("  {}  (latest in {})", v, feature_band(v));
    }
    println!();

    if args.dry_run {
        return Ok(());
    }

    if !args.yes {
        let proceed = confirm(
            &format!("Remove {} outdated SDK version(s)?", to_remove.len()),
            false,
        )?;
        if !proceed {
            println!("Aborted.");
            return Ok(());
        }
    }

    let mut freed: u64 = 0;
    for v in &to_remove {
        let dir = paths.sdk_dir.join(v);
        freed += dir_size(&dir).unwrap_or(0);
        std::fs::remove_dir_all(&dir)
            .with_context(|| format!("Failed to remove {}", dir.display()))?;
    }

    println!(
        "Removed {} outdated SDK version(s). Freed {}.",
        to_remove.len(),
        human_bytes(freed)
    );
    Ok(())
}

/// Recursively sum the byte size of a directory's files.
fn dir_size(path: &Path) -> AnyResult<u64> {
    let mut total = 0;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_dir() {
            total += dir_size(&entry.path()).unwrap_or(0);
        } else {
            total += meta.len();
        }
    }
    Ok(total)
}

/// Format a byte count as a human-readable string (e.g. "1.2 GB").
fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}
