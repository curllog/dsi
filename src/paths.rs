use anyhow::{Context, Ok, Result as AnyResult};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DsiPaths {
    /// Where .NET SDKs are installed: ~/.dotnet
    pub dotnet_root: PathBuf,
    /// The dotnet muxer binary: ~/.dotnet/dotnet
    pub dotnet_bin: PathBuf,
    /// Where SDKs live: ~/.dotnet/sdk/
    pub sdk_dir: PathBuf,
}

impl DsiPaths {
    pub fn has_dotnet(&self) -> bool {
        self.dotnet_bin.exists()
    }

    pub fn resolve() -> AnyResult<Self> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        let dotnet_root = home.join(".dotnet");
        let dotnet_bin = dotnet_root.join(if cfg!(windows) {
            "dotnet.exe"
        } else {
            "dotnet"
        });
        let sdk_dir = dotnet_root.join("sdk");
        Ok(DsiPaths {
            dotnet_root,
            dotnet_bin,
            sdk_dir,
        })
    }

    pub fn installed_sdks(&self) -> AnyResult<Vec<String>> {
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
