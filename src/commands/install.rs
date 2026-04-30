use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result as AnyResult, bail};
use clap::Parser;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha512};

use crate::api::client::{ApiClient, find_sdk_file_for_rid};
use crate::api::models::{
    Channel, ChannelReleases, MIN_SUPPORTED_MAJOR_VERSION, ReleasesIndex, Sdk,
};
use crate::paths::DsiPaths;
use crate::platform::Platform;

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
    // Step 1: Decide what to install
    let target = resolve_target(&args)?;

    // Step 2: Fetch the release index and find the channel
    let client = ApiClient::new()?;
    let index = client.fetch_releases_index().await?;

    let channel = pick_channel(&index, &target)?;
    check_channel_supported(channel)?;

    // Step 3: Fetch the channel's detailed release info
    println!(
        "Resolving {} from channel {}...",
        target, channel.channel_version
    );
    let releases = client
        .fetch_channel_releases(&channel.releases_json_url)
        .await?;

    // Step 4: Find the specific SDK to install
    let sdk = pick_sdk(&releases, &target)?;

    // Step 5: Find the download for our platform
    let platform = Platform::detect();
    let rid = platform.rid();
    let file = find_sdk_file_for_rid(&sdk.files, &rid)
        .with_context(|| format!("No download available for platform {}", rid))?;

    println!("Installing SDK {} for {}", sdk.version, rid);

    // Step 6: Download
    let paths = DsiPaths::resolve()?;
    std::fs::create_dir_all(&paths.dotnet_root)
        .with_context(|| format!("Failed to create {}", paths.dotnet_root.display()))?;

    let archive_filename = format!("dsi-download-{}-{}.tar.gz", sdk.version, rid);
    let archive_path = paths.dotnet_root.join(&archive_filename);
    download_file(&client, &file.url, &archive_path).await?;

    // Step 7: Verify checksum
    verify_sha512(&archive_path, &file.hash)?;

    // Step 8: Extract
    extract_tarball(&archive_path, &paths.dotnet_root)?;

    // Step 9: Clean up the downloaded archive
    std::fs::remove_file(&archive_path).ok();

    println!();
    println!(
        "✓ Installed SDK {} to {}",
        sdk.version,
        paths.dotnet_root.display()
    );
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
/// Pick the right SDK from a channel's releases.
fn pick_sdk<'a>(releases: &'a ChannelReleases, target: &str) -> AnyResult<&'a Sdk> {
    // If the target looks like a full version (e.g., "9.0.312"), find that exact one.
    // Otherwise, return the latest SDK in the channel.
    let is_full_version = target.matches('.').count() >= 2;

    if is_full_version {
        releases
            .releases
            .iter()
            .find(|r| r.sdk.version == target)
            .map(|r| &r.sdk)
            .with_context(|| format!("SDK version {} not found in channel", target))
    } else {
        // Latest is the first entry (releases are ordered newest-first)
        releases
            .releases
            .first()
            .map(|r| &r.sdk)
            .context("Channel contains no releases")
    }
}

// ─── Channel resolution ───────────────────────────────────────────────────

/// Pick the right channel from the index based on the target string.
fn pick_channel<'a>(index: &'a ReleasesIndex, target: &str) -> AnyResult<&'a Channel> {
    let channel = match target {
        "lts" => index
            .channels
            .iter()
            .filter(|c| c.is_lts() && c.is_supported())
            .max_by_key(|c| c.major_version()),

        "latest" => index
            .channels
            .iter()
            .filter(|c| c.is_supported())
            .max_by_key(|c| c.major_version()),

        version => {
            // Match either a channel ("9.0") or a full version ("9.0.312")
            let channel_key = extract_channel_version(version);
            index
                .channels
                .iter()
                .find(|c| c.channel_version == channel_key)
        }
    };

    channel.with_context(|| format!("Could not find channel matching '{}'", target))
}

/// Extract the major.minor channel from a version string.
///   "9.0"      → "9.0"
///   "9.0.312"  → "9.0"
fn extract_channel_version(version: &str) -> String {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        format!("{}.{}", parts[0], parts[1])
    } else {
        version.to_string()
    }
}

/// Reject channels below .NET 6 with a friendly error.
fn check_channel_supported(channel: &Channel) -> AnyResult<()> {
    if !channel.is_installable() {
        bail!(
            ".NET {} is below the minimum supported version (.NET {}).\n\
             Older versions require OpenSSL 1.x which modern Linux distros no longer ship.",
            channel.channel_version,
            MIN_SUPPORTED_MAJOR_VERSION
        );
    }
    Ok(())
}

// ─── Download ─────────────────────────────────────────────────────────────

async fn download_file(client: &ApiClient, url: &str, dest: &Path) -> AnyResult<()> {
    let response = client.download(url).await?;
    let total_size = response.content_length().unwrap_or(0);

    // Set up the progress bar
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::with_template(
            "  {spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})",
        )
        .unwrap()
        .progress_chars("=> "),
    );

    // Open the destination file for writing
    let mut file = std::fs::File::create(dest)
        .with_context(|| format!("Failed to create {}", dest.display()))?;

    // Stream the response body to disk, updating progress
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Download stream error")?;
        file.write_all(&chunk)
            .context("Failed to write chunk to disk")?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message("Download complete");
    Ok(())
}

// ─── Verification ─────────────────────────────────────────────────────────

fn verify_sha512(file_path: &Path, expected_hex: &str) -> AnyResult<()> {
    use std::io::Read;

    print!("Verifying checksum... ");
    std::io::stdout().flush().ok();

    let mut file = std::fs::File::open(file_path)
        .with_context(|| format!("Failed to open {} for hashing", file_path.display()))?;

    let mut hasher = Sha512::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = file.read(&mut buffer).context("Failed to read file")?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let computed = hasher.finalize();
    let computed_hex = hex::encode(computed);

    if computed_hex.eq_ignore_ascii_case(expected_hex) {
        println!("✓");
        Ok(())
    } else {
        bail!(
            "Checksum mismatch!\n  expected: {}\n  got:      {}",
            expected_hex,
            computed_hex
        )
    }
}

// ─── Extraction ───────────────────────────────────────────────────────────

fn extract_tarball(archive: &Path, dest: &Path) -> AnyResult<()> {
    print!("Extracting... ");
    std::io::stdout().flush().ok();

    let file = std::fs::File::open(archive)
        .with_context(|| format!("Failed to open {}", archive.display()))?;

    let decompressed = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decompressed);

    archive
        .unpack(dest)
        .with_context(|| format!("Failed to extract to {}", dest.display()))?;

    println!("✓");
    Ok(())
}
