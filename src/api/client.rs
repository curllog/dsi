use super::models::ReleasesIndex;
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
}
