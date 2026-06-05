// src/commands/uninstall.rs
//
// `dsi uninstall <version>` — Remove a specific SDK version.

use anyhow::{Context, Result as AnyResult, bail};
use clap::Parser;

use crate::paths::DsiPaths;
use crate::prompt::confirm;

#[derive(Parser)]
pub struct UninstallArgs {
    /// Exact SDK version to remove (e.g. "8.0.300")
    pub version: String,
    /// Skip the confirmation prompt
    #[arg(short, long)]
    pub yes: bool,
}

pub async fn run(args: UninstallArgs) -> AnyResult<()> {
    let paths = DsiPaths::resolve()?;
    let target = paths.sdk_dir.join(&args.version);

    if !target.is_dir() {
        bail!(
            "SDK {} is not installed (no directory at {}).",
            args.version,
            target.display()
        );
    }

    if !args.yes {
        let proceed = confirm(
            &format!("Remove SDK {} from {}?", args.version, target.display()),
            false,
        )?;
        if !proceed {
            println!("Aborted.");
            return Ok(());
        }
    }

    std::fs::remove_dir_all(&target)
        .with_context(|| format!("Failed to remove {}", target.display()))?;

    println!("✓ Removed SDK {}", args.version);
    println!();
    println!("Note: shared runtimes were left in place — other SDKs may depend on them.");
    Ok(())
}
