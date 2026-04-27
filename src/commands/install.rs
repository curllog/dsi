use anyhow::{Result as AnyResult, bail};
use clap::Parser;

#[derive(Parser)]
pub struct InstallArgs {
    /// Version to install: channel ("9.0") or exact ("9.0.312")
    pub version: Option<String>,
    /// Install the latest LTS version
    #[arg(long)]
    pub lts: bool,
    /// Install the latest supported version (LTS or STS)
    #[arg(long)]
    pub latest: bool,
}

pub async fn run(args: InstallArgs) -> AnyResult<()> {
    let target = resolve_target(&args)?;
    println!("Would install: {}", target);
    Ok(())
}

/// Decide what the user wants to install.
fn resolve_target(args: &InstallArgs) -> AnyResult<String> {
    if args.lts {
        Ok("lts".to_string())
    } else if args.latest {
        Ok("latest".to_string())
    } else if let Some(version) = &args.version {
        Ok(version.clone())
    } else {
        bail!(
            "Specify a version to install.\n\n\
             Examples:\n  \
             dsi install 9.0          Install latest .NET 9 SDK\n  \
             dsi install 9.0.312      Install specific SDK version\n  \
             dsi install --lts        Install latest LTS version\n  \
             dsi install --latest     Install latest supported version"
        )
    }
}
