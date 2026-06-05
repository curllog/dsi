// src/commands/selfupdate.rs
//
// `dsi selfupdate` — Update the dsi binary itself.
//
// NOT YET IMPLEMENTED. The release/asset convention (how binaries are
// published to GitHub releases at curllog/dsi) is still undecided and must be
// settled together with the release CI and `install.sh`. Until then this is a
// deliberate stub rather than a guessed implementation.

use anyhow::{Result as AnyResult, bail};
use clap::Parser;

#[derive(Parser)]
pub struct SelfUpdateArgs {
    /// Skip the confirmation prompt (reserved for the future implementation)
    #[arg(short, long)]
    pub yes: bool,
}

pub async fn run(_args: SelfUpdateArgs) -> AnyResult<()> {
    bail!(
        "selfupdate is not implemented yet.\n\
         Reinstall the latest dsi via install.sh for now."
    )
}
