use crate::{
    USER_AGENT,
    utils::{ClientExt, ReqwestExtError, ResultExt},
};
use bytes::Bytes;
use reqwest::{Client, ClientBuilder, header, header::HeaderMap};
use semver::Version;
use serde::Deserialize;
use serde_with::{DisplayFromStr, Map, serde_as};
use ssri::Integrity;
use std::sync::LazyLock;
use url::Url;

#[derive(Debug)]
pub struct NpmClient<'a> {
    registries: &'a [Url],
}

#[derive(Debug, thiserror::Error)]
pub enum NpmError {
    #[error(transparent)]
    Http(#[from] ReqwestExtError),
    #[error("npm metadata missing latest version")]
    MissingLatestVersion,
    #[error("downloaded file integrity check failed: {0}")]
    Integrity(#[from] ssri::Error),
}

#[serde_as]
#[derive(Deserialize)]
struct Metadata {
    #[serde(rename = "dist-tags")]
    dist_tags: DistTags,
    #[serde_as(as = "Map<_, _>")]
    versions: Vec<(Version, VersionMetadata)>,
}

#[derive(Deserialize)]
struct DistTags {
    latest: Version,
}
#[serde_as]
#[derive(Debug, Deserialize)]
pub struct VersionMetadata {
    pub version: Version,
    #[serde(rename = "githubTag")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub github_tag: Option<u64>,
    pub dist: DistInfo,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct DistInfo {
    #[serde_as(as = "DisplayFromStr")]
    integrity: Integrity,
    tarball: Url,
}

impl<'a> NpmClient<'a> {
    /// Create a new NpmClient with the given registries.
    pub fn new(registries: &'a [Url]) -> Self {
        NpmClient { registries }
    }

    /// Download a distribution file.
    pub async fn download_dist(&self, dist: DistInfo) -> Result<Bytes, NpmError> {
        let bytes = NPM_CLIENT
            .download([dist.tarball].into_iter())
            .await
            .inspect_err(|e| error!("error downloading dist file: {e}"))?;

        dist.integrity.check(&bytes)?;
        Ok(bytes)
    }

    pub async fn get_lastest_version(&self, package: &str) -> Result<VersionMetadata, NpmError> {
        let metadata = NPM_CLIENT
            .get_json::<_, Metadata>(
                self.registries
                    .iter()
                    .map(|base_url| base_url.join(package).infallible()),
            )
            .await
            .inspect_err(|e| error!("error fetching npm metadata: {e}"))?;

        metadata
            .versions
            .into_iter()
            .find(|(version, _)| version == &metadata.dist_tags.latest)
            .ok_or(NpmError::MissingLatestVersion)
            .map(|(_, meta)| meta)
            .inspect_err(|e| error!("error finding latest version in metadata: {e}"))
    }
}

static NPM_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    ClientBuilder::default()
        .user_agent(*USER_AGENT)
        .default_headers(HeaderMap::from_iter([
            (header::FROM, "ligh.tsing@gmail.com".parse().infallible()),
            (
                header::ACCEPT,
                "application/vnd.npm.install-v1+json; q=1.0, application/json; q=0.8, */*"
                    .parse()
                    .infallible(),
            ),
        ]))
        .build()
        .expect("Failed to build default NPM client")
});

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_get_npm_metadata() {
        let registries = crate::config::default_npm_registries();
        let npm_client = NpmClient::new(&registries);

        npm_client
            .get_lastest_version("@lightsing/llc-zh-cn")
            .await
            .unwrap();
    }

    #[test]
    async fn test_download_dist() {
        let registries = crate::config::default_npm_registries();
        let npm_client = NpmClient::new(&registries);

        let meta = npm_client
            .get_lastest_version("@lightsing/llc-zh-cn")
            .await
            .unwrap();

        npm_client.download_dist(meta.dist).await.unwrap();
    }
}
