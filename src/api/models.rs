use serde::Deserialize;
pub const MIN_SUPPORTED_MAJOR_VERSION: u32 = 6;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ChannelReleases {
    pub channel_version: String,
    pub latest_release: String,
    pub latest_sdk: String,
    pub releases: Vec<Release>,
}

/// A single release in the channel.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Release {
    pub release_date: String,
    pub release_version: String,
    pub sdk: Sdk,
}

/// SDK metadata including download URLs for each platform.
#[derive(Debug, Deserialize)]
pub struct Sdk {
    pub version: String,
    #[serde(default)]
    pub files: Vec<SdkFile>,
}

/// A downloadable SDK file for a specific platform.
#[derive(Debug, Deserialize)]
pub struct SdkFile {
    pub name: String,
    pub rid: String,
    pub url: String,
    pub hash: String,
}

#[derive(Debug, Deserialize)]
pub struct ReleasesIndex {
    #[serde(rename = "releases-index")]
    pub channels: Vec<Channel>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Channel {
    /// Channel version: "9.0", "10.0"
    pub channel_version: String,

    /// Latest release in this channel: "9.0.14"
    pub latest_release: String,

    /// Date of latest release: "2026-03-10"
    pub latest_release_date: String,

    /// Latest SDK version: "9.0.312"
    pub latest_sdk: String,

    /// "lts" or "sts"
    pub release_type: String,

    /// "active", "maintenance", "eol", "preview"
    pub support_phase: String,

    /// End of life date (may be missing for preview channels)
    pub eol_date: Option<String>,

    #[serde(rename = "releases.json")]
    pub releases_json_url: String,
}

impl Channel {
    pub fn is_lts(&self) -> bool {
        self.release_type == "lts"
    }
    pub fn is_supported(&self) -> bool {
        self.support_phase != "eol"
    }
    pub fn major_version(&self) -> u32 {
        self.channel_version
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }
    pub fn is_installable(&self) -> bool {
        self.major_version() >= MIN_SUPPORTED_MAJOR_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_release_index() {
        let json = r#"{
                "releases-index": [
                    {
                        "channel-version": "10.0",
                        "latest-release": "10.0.5",
                        "latest-release-date": "2026-03-12",
                        "latest-sdk": "10.0.201",
                        "release-type": "lts",
                        "support-phase": "active",
                        "eol-date": "2028-11-14",
                        "releases.json": "https://builds.dotnet.microsoft.com/dotnet/release-metadata/10.0/releases.json"
                    }
                ]
            }"#;

        let parsed: ReleasesIndex = serde_json::from_str(json).expect("failed to parse JSON");

        assert_eq!(parsed.channels.len(), 1);

        let channel = &parsed.channels[0];
        assert_eq!(channel.channel_version, "10.0");
        assert_eq!(channel.latest_sdk, "10.0.201");
        assert_eq!(channel.release_type, "lts");
        assert_eq!(channel.support_phase, "active");
        assert_eq!(channel.eol_date, Some("2028-11-14".to_string()));
    }
}
