// src/commands/update.rs
//
// `dsi update` — Update installed SDKs to the latest patch in their channel,
// removing the patch each update supersedes.

use std::collections::BTreeMap;

use anyhow::{Context, Result as AnyResult};
use clap::Parser;

use crate::api::client::ApiClient;
use crate::api::models::ReleasesIndex;
use crate::commands::install::fetch_and_install;
use crate::paths::DsiPaths;
use crate::prompt::confirm;
use crate::version::{channel_of, version_key};

#[derive(Parser)]
pub struct UpdateArgs {
    /// Skip the confirmation prompt
    #[arg(short, long)]
    pub yes: bool,
}

/// A pending update: bump the highest installed patch in a channel up to the
/// latest patch the channel offers.
struct Plan {
    channel: String,
    from: String,
    to: String,
}

pub async fn run(args: UpdateArgs) -> AnyResult<()> {
    let paths = DsiPaths::resolve()?;
    let installed = paths.installed_sdks()?;

    if installed.is_empty() {
        println!();
        println!("  No .NET SDKs installed — nothing to update.");
        println!("  Run `dsi install --lts` to install one.");
        println!();
        return Ok(());
    }

    // Group installed SDKs by their channel ("9.0"), tracking the highest patch.
    let mut current_by_channel: BTreeMap<String, String> = BTreeMap::new();
    for version in &installed {
        let channel = channel_of(version);
        current_by_channel
            .entry(channel)
            .and_modify(|cur| {
                if version_key(version) > version_key(cur) {
                    *cur = version.clone();
                }
            })
            .or_insert_with(|| version.clone());
    }

    println!("Checking for updates...");

    let client = ApiClient::new()?;
    let index = client.fetch_releases_index().await?;

    // Build the set of all installed versions for "already installed" checks.
    let installed_set: std::collections::HashSet<&String> = installed.iter().collect();

    let mut plans: Vec<Plan> = Vec::new();
    for (channel, current) in &current_by_channel {
        let Some(latest) = latest_sdk_for_channel(&index, channel) else {
            continue;
        };

        // An update exists only if the channel's latest patch isn't installed
        // yet and is actually newer than what we have.
        if !installed_set.contains(&latest) && version_key(&latest) > version_key(current) {
            println!("  {} → {}  (available)", current, latest);
            plans.push(Plan {
                channel: channel.clone(),
                from: current.clone(),
                to: latest,
            });
        }
    }

    if plans.is_empty() {
        println!();
        println!("All SDKs are up to date.");
        return Ok(());
    }

    if !args.yes {
        let proceed = confirm("\nInstall updates?", true)?;
        if !proceed {
            println!("Aborted.");
            return Ok(());
        }
    }

    for plan in &plans {
        println!();
        // Re-fetch the channel's releases to locate the SDK's download files.
        let channel = index
            .channels
            .iter()
            .find(|c| c.channel_version == plan.channel)
            .with_context(|| format!("Channel {} disappeared from the index", plan.channel))?;
        let releases = client
            .fetch_channel_releases(&channel.releases_json_url)
            .await?;
        let sdk = releases
            .releases
            .iter()
            .map(|r| &r.sdk)
            .find(|s| s.version == plan.to)
            .with_context(|| format!("SDK {} not found in channel releases", plan.to))?;

        fetch_and_install(&client, sdk, &paths).await?;

        // Remove the patch this update superseded so old versions don't
        // accumulate. Shared runtimes are left in place; failure to remove
        // is non-fatal since the new SDK is already installed.
        let old_dir = paths.sdk_dir.join(&plan.from);
        match std::fs::remove_dir_all(&old_dir) {
            Ok(()) => println!(
                "✓ Updated {} → {} (removed {})",
                plan.from, plan.to, plan.from
            ),
            Err(e) => {
                println!("✓ Updated {} → {}", plan.from, plan.to);
                eprintln!(
                    "warning: could not remove old SDK {}: {} — run `dsi prune` to clean up",
                    plan.from, e
                );
            }
        }
    }

    Ok(())
}

/// Resolve the latest SDK version for a channel, or `None` if the channel
/// isn't present in the index.
fn latest_sdk_for_channel(index: &ReleasesIndex, channel: &str) -> Option<String> {
    index
        .channels
        .iter()
        .find(|c| c.channel_version == channel)
        .map(|c| c.latest_sdk.clone())
}
