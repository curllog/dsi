use serde::Deserialize;
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
                    "eol-date": "2028-11-14"
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
