// src/commands/selfupdate.rs
//
// `dsi selfupdate` — Update the dsi binary itself.
//
// Downloads the latest release asset (`dsi-{rid}.tar.gz`) from GitHub Releases
// at `curllog/dsi`, verifies its SHA-256 checksum, and atomically replaces the
// running binary. Asset/tag convention matches the release CI and `install.sh`.

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use anyhow::{Context, Result as AnyResult, bail};
use clap::Parser;
use futures_util::StreamExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::platform::Platform;
use crate::prompt::confirm;
use crate::version::version_key;

const REPO: &str = "curllog/dsi";

#[derive(Parser)]
pub struct SelfUpdateArgs {
    /// Skip the confirmation prompt
    #[arg(short, long)]
    pub yes: bool,
}

/// Minimal view of the GitHub "latest release" response.
#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
}

pub async fn run(args: SelfUpdateArgs) -> AnyResult<()> {
    let current = env!("CARGO_PKG_VERSION");

    let http = reqwest::Client::builder()
        .user_agent(format!("dsi/{}", current))
        .build()
        .context("Failed to create HTTP client")?;

    // 1. Find the latest release tag.
    println!("Checking for updates...");
    let latest_tag = fetch_latest_tag(&http).await?;
    let latest = latest_tag.strip_prefix('v').unwrap_or(&latest_tag);

    // 2. Compare versions.
    if version_key(latest) <= version_key(current) {
        println!("dsi is already up to date ({}).", current);
        return Ok(());
    }

    println!("Update available: {} → {}", current, latest);
    if !args.yes && !confirm("Update now?", true)? {
        println!("Aborted.");
        return Ok(());
    }

    // 3. Resolve the asset for this platform.
    let rid = Platform::detect().rid();
    let asset = format!("dsi-{}.tar.gz", rid);
    let base = format!(
        "https://github.com/{}/releases/download/{}",
        REPO, latest_tag
    );

    // 4. Stage everything in a temp dir and clean up afterwards.
    let work = std::env::temp_dir().join(format!("dsi-selfupdate-{}", std::process::id()));
    std::fs::create_dir_all(&work)
        .with_context(|| format!("Failed to create {}", work.display()))?;
    let result = perform_update(&http, &base, &asset, &rid, &work).await;
    std::fs::remove_dir_all(&work).ok();
    result?;

    println!("✓ Updated dsi {} → {}", current, latest);
    Ok(())
}

/// Download, verify, extract, and swap in the new binary.
async fn perform_update(
    http: &reqwest::Client,
    base: &str,
    asset: &str,
    rid: &str,
    work: &Path,
) -> AnyResult<()> {
    let archive_path = work.join(asset);

    println!("Downloading {}...", asset);
    download(http, &format!("{}/{}", base, asset), &archive_path)
        .await
        .with_context(|| format!("No prebuilt binary available for platform {}", rid))?;

    let sha_text = http
        .get(format!("{}/{}.sha256", base, asset))
        .send()
        .await
        .context("Failed to download checksum")?
        .error_for_status()
        .context("Checksum download failed")?
        .text()
        .await
        .context("Failed to read checksum")?;

    verify_sha256(&archive_path, &sha_text)?;

    extract_tarball(&archive_path, work)?;
    let new_binary = work.join("dsi");
    if !new_binary.exists() {
        bail!("archive did not contain a dsi binary");
    }

    replace_current_exe(&new_binary)?;
    Ok(())
}

/// Query GitHub for the latest release tag (e.g. "v0.1.0").
async fn fetch_latest_tag(http: &reqwest::Client) -> AnyResult<String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let release: GhRelease = http
        .get(&url)
        .send()
        .await
        .context("Failed to query GitHub releases — are you online?")?
        .error_for_status()
        .context("GitHub releases API request failed")?
        .json()
        .await
        .context("Failed to parse GitHub release JSON")?;
    Ok(release.tag_name)
}

/// Stream a URL to a file on disk.
async fn download(http: &reqwest::Client, url: &str, dest: &Path) -> AnyResult<()> {
    let resp = http
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to download {}", url))?
        .error_for_status()
        .with_context(|| format!("Download failed for {}", url))?;

    let mut file = std::fs::File::create(dest)
        .with_context(|| format!("Failed to create {}", dest.display()))?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Download stream error")?;
        file.write_all(&chunk).context("Failed to write to disk")?;
    }
    Ok(())
}

/// Verify a file against the hash in a `sha256sum`-style checksum file
/// (the first whitespace-delimited token is the hex digest).
fn verify_sha256(file_path: &Path, sha_file_contents: &str) -> AnyResult<()> {
    use std::io::Read;

    let expected = sha_file_contents
        .split_whitespace()
        .next()
        .context("Empty checksum file")?;

    print!("Verifying checksum... ");
    std::io::stdout().flush().ok();

    let mut file = std::fs::File::open(file_path)
        .with_context(|| format!("Failed to open {} for hashing", file_path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer).context("Failed to read file")?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let computed = hex::encode(hasher.finalize());

    if computed.eq_ignore_ascii_case(expected) {
        println!("✓");
        Ok(())
    } else {
        bail!(
            "Checksum mismatch!\n  expected: {}\n  got:      {}",
            expected,
            computed
        )
    }
}

fn extract_tarball(archive: &Path, dest: &Path) -> AnyResult<()> {
    let file = std::fs::File::open(archive)
        .with_context(|| format!("Failed to open {}", archive.display()))?;
    let decompressed = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decompressed);
    archive
        .unpack(dest)
        .with_context(|| format!("Failed to extract to {}", dest.display()))?;
    Ok(())
}

/// Atomically replace the running binary with `new_binary`.
///
/// The new binary is staged in the *same* directory as the current executable
/// so the final `rename` is atomic (a cross-filesystem rename would fail).
/// Renaming over a running executable is safe on Unix — the running process
/// keeps the old inode until it exits.
fn replace_current_exe(new_binary: &Path) -> AnyResult<()> {
    let exe = std::env::current_exe().context("Could not determine the dsi binary location")?;
    let dir = exe.parent().context("dsi binary has no parent directory")?;

    let staged = dir.join(format!(".dsi-update-{}", std::process::id()));
    std::fs::copy(new_binary, &staged)
        .with_context(|| format!("Failed to stage new binary in {}", dir.display()))?;

    // rwxr-xr-x
    let mut perms = std::fs::metadata(&staged)
        .context("Failed to read staged binary metadata")?
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&staged, perms).context("Failed to set executable permissions")?;

    if let Err(e) = std::fs::rename(&staged, &exe) {
        std::fs::remove_file(&staged).ok();
        return Err(e).with_context(|| {
            format!(
                "Failed to replace {} (insufficient permissions?)",
                exe.display()
            )
        });
    }

    Ok(())
}
