use crate::{
    utils,
    zeroasso::{CLIENT, utils::request_zeroasso_api},
};
use futures::{FutureExt, TryFutureExt};
use llc_rs::LLCConfig;
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use std::sync::Arc;

#[instrument(skip(llc_config), level = "trace", ret)]
#[inline]
pub async fn run(llc_config: Arc<LLCConfig>) -> eyre::Result<u64> {
    utils::select_ok2(
        get_version_github(&llc_config),
        get_version_zeroasso(&llc_config),
    )
    .await
}

#[inline]
async fn get_version_github(llc_config: &LLCConfig) -> eyre::Result<u64> {
    #[serde_as]
    #[derive(Deserialize)]
    pub struct GithubRelease {
        #[serde_as(as = "DisplayFromStr")]
        tag_name: u64,
    }

    let ver: GithubRelease = CLIENT
        .get(
            llc_config
                .github()
                .api_url()
                .join("v2/resource/get_version")
                .expect("infallible"),
        )
        .send()
        .map(|r| r.and_then(|res| res.error_for_status()))
        .and_then(|res| res.json())
        .await?;
    info!("get version from GitHub API");
    Ok(ver.tag_name)
}

#[instrument(skip(llc_config), level = "trace")]
#[inline]
async fn get_version_zeroasso(llc_config: &LLCConfig) -> eyre::Result<u64> {
    #[derive(Deserialize)]
    pub struct LLCVersion {
        version: u64,
    }
    let res: LLCVersion = request_zeroasso_api(llc_config, "v2/resource/get_version").await?;
    Ok(res.version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use llc_rs::LLCConfig;

    #[tokio::test]
    async fn test_get_version() {
        let llc_config = Arc::new(LLCConfig::default());
        let version = run(llc_config).await;
        assert!(version.is_ok());
        println!("[test_get_version] Version: {}", version.unwrap());
    }
}
