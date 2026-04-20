use crate::api::client::ApiClient;
use anyhow::Result as AnyResult;
use clap::Parser;

#[derive(Parser)]
pub struct LsRemoteArgs {
    /// Only show LTS releases
    #[arg(long)]
    pub lts: bool,
    /// Include end-of-life versions
    #[arg(long)]
    pub include_eol: bool,
}

pub async fn run(args: LsRemoteArgs) -> AnyResult<()> {
    let client = ApiClient::new()?;
    let index = client.fetch_releases_index().await?;
    println!();
    println!(
        "  {:<10} {:<44} {:<10} {:<14} {}",
        "Channel", "Latest SDK", "Type", "Status", r#"EOL Date"#
    );
    println!("  {}", "─".repeat(100));

    for channel in &index.channels {
        if !args.include_eol && channel.support_phase == "eol" {
            continue;
        }

        if args.lts && channel.release_type != "lts" {
            continue;
        }
        let eol = channel.eol_date.as_deref().unwrap_or("-");

        println!(
            "  {:<10} {:<44} {:<10} {:<14} {}",
            channel.channel_version,
            channel.latest_sdk,
            channel.release_type,
            channel.support_phase,
            eol,
        );
    }

    println!();
    Ok(())
}
