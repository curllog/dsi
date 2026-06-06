// src/commands/selfuninstall.rs
//
// `dsi selfuninstall` — Remove the dsi binary itself, leaving SDKs intact.

use anyhow::{Context, Result as AnyResult, bail};
use clap::Parser;

use crate::paths::DsiPaths;
use crate::prompt::confirm;

#[derive(Parser)]
pub struct SelfUninstallArgs {
    /// Skip the confirmation prompt
    #[arg(short, long)]
    pub yes: bool,
}

pub async fn run(args: SelfUninstallArgs) -> AnyResult<()> {
    let exe = std::env::current_exe().context("Could not determine the dsi binary location")?;
    let paths = DsiPaths::resolve()?;

    println!("This will remove dsi from {}.", exe.display());
    if paths.dotnet_root.exists() {
        println!(
            "SDKs in {} will NOT be removed.",
            paths.dotnet_root.display()
        );
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

    println!("✓ Removed dsi");
    Ok(())
}
