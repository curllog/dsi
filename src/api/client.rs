use super::models::{ChannelReleases, ReleasesIndex, SdkFile};
use anyhow::{Context, Result as AnyResult, bail};
const RELEASES_INDEX_URL: &str =
    "https://builds.dotnet.microsoft.com/dotnet/release-metadata/releases-index.json";

pub struct ApiClient {
    http: reqwest::Client,
}

impl ApiClient {
    pub fn new() -> AnyResult<Self> {
        let http = reqwest::Client::builder()
            .user_agent(format!("dsi/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .context("Failed to Create Http Client")?;

        Ok(ApiClient { http })
    }

    pub async fn fetch_releases_index(&self) -> AnyResult<ReleasesIndex> {
        let response = self
            .http
            .get(RELEASES_INDEX_URL)
            .send()
            .await
            .context("Failed to fetrch releases index - are you online?")?;

        let status = response.status();
        if !status.is_success() {
            bail!("Releases api returned http {}", status);
        }

        let index: ReleasesIndex = response
            .json()
            .await
            .context("Failed to parse releases index JSON")?;
        Ok(index)
    }

    /// Fetch detailed release info for a specific channel.
    pub async fn fetch_channel_releases(
        &self,
        channel_url: &str,
    ) -> anyhow::Result<ChannelReleases> {
        let response = self
            .http
            .get(channel_url)
            .send()
            .await
            .context("Failed to fetch channel releases")?;

        let status = response.status();
        if !status.is_success() {
            bail!("Channel releases API returned HTTP {}", status);
        }

        let releases: ChannelReleases = response
            .json()
            .await
            .context("Failed to parse channel releases JSON")?;

        Ok(releases)
    }

    /// Start a download. Returns the streaming response so the caller can
    /// read the body chunk-by-chunk (typically with a progress bar).
    pub async fn download(&self, url: &str) -> AnyResult<reqwest::Response> {
        let response = self
            .http
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to start download from {}", url))?;

        let status = response.status();
        if !status.is_success() {
            bail!("Download returned HTTP {} from {}", status, url);
        }

        Ok(response)
    }
}

/// Find the SDK file matching a specific platform RID.
pub fn find_sdk_file_for_rid<'a>(files: &'a [SdkFile], rid: &str) -> Option<&'a SdkFile> {
    files.iter().find(|f| f.rid == rid)
}
