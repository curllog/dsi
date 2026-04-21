// src/commands/ls.rs
//
// `dsi ls` — List locally installed SDK versions.

use anyhow::Result;
use clap::Parser;

use crate::paths::DsiPaths;

#[derive(Parser)]
pub struct LsArgs;

pub async fn run(_args: LsArgs) -> Result<()> {
    let paths = DsiPaths::resolve()?;
    let sdks = paths.installed_sdks()?;

    if sdks.is_empty() {
        println!();
        println!("  No .NET SDKs installed.");
        println!("  Run `dsi install --lts` to install one.");
        println!();
        return Ok(());
    }

    println!();
    println!("  Installed SDKs (in {}):", paths.sdk_dir.display());
    println!();
    println!("  {:<20} {}", "SDK Version", "Channel");
    println!("  {}", "─".repeat(32));

    for sdk in &sdks {
        let channel = extract_channel(sdk);
        println!("  {:<20} {}", sdk, channel);
    }

    println!();

    if paths.has_dotnet() {
        if let Ok(output) = std::process::Command::new(&paths.dotnet_bin)
            .arg("--version")
            .output()
        {
            if output.status.success() {
                let active = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("  Active SDK: {}", active);
                println!();
            }
        }
    }

    Ok(())
}

fn extract_channel(version: &str) -> String {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        format!("{}.{}", parts[0], parts[1])
    } else {
        version.to_string()
    }
}
