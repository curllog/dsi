// src/paths.rs
//
// Centralized path definitions for dsi.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// All paths dsi uses.
#[derive(Debug, Clone)]
pub struct DsiPaths {
    /// Root install directory: ~/.dotnet
    pub dotnet_root: PathBuf,

    /// The dotnet muxer binary: ~/.dotnet/dotnet
    pub dotnet_bin: PathBuf,

    /// Where SDKs live: ~/.dotnet/sdk/
    pub sdk_dir: PathBuf,
}

impl DsiPaths {
    /// Resolve paths based on the user's home directory.
    pub fn resolve() -> Result<Self> {
        let home = dirs::home_dir().context("Could not determine home directory")?;

        let dotnet_root = home.join(".dotnet");
        let dotnet_bin = dotnet_root.join("dotnet");
        let sdk_dir = dotnet_root.join("sdk");

        Ok(DsiPaths {
            dotnet_root,
            dotnet_bin,
            sdk_dir,
        })
    }

    /// Check if at least one SDK has been installed (muxer exists).
    pub fn has_dotnet(&self) -> bool {
        self.dotnet_bin.exists()
    }

    /// List installed SDK version strings by scanning ~/.dotnet/sdk/.
    pub fn installed_sdks(&self) -> Result<Vec<String>> {
        if !self.sdk_dir.exists() {
            return Ok(Vec::new());
        }

        let mut versions: Vec<String> = std::fs::read_dir(&self.sdk_dir)
            .with_context(|| format!("Failed to read SDK directory: {}", self.sdk_dir.display()))?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().to_string_lossy().to_string();

                if entry.file_type().ok()?.is_dir() && name.contains('.') {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();

        versions.sort();
        Ok(versions)
    }
}
