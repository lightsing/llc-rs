use crate::{
    LLCConfig,
    utils::OptionExt,
    zeroasso::{ZeroAssoApiError, get_github_client, request_zeroasso_api},
};
use futures_util::{FutureExt, StreamExt, TryFutureExt, stream::FuturesUnordered};
use nyquest::Request;
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use std::{future::ready, sync::Arc};

#[instrument(skip(llc_config), level = "trace", ret)]
#[inline]
pub async fn run(llc_config: Arc<LLCConfig>) -> Result<u64, ZeroAssoApiError> {
    [
        get_version_github(&llc_config).map(|r| (r, false)).boxed(),
        get_version_zeroasso(&llc_config).map(|r| (r, true)).boxed(),
    ]
    .into_iter()
    .collect::<FuturesUnordered<_>>()
    .skip_while(|(r, is_last)| ready(!is_last && r.is_err()))
    .next()
    .await
    .infallible()
    .0
}

#[inline]
async fn get_version_github(llc_config: &LLCConfig) -> Result<u64, ZeroAssoApiError> {
    #[serde_as]
    #[derive(Deserialize)]
    pub struct GithubRelease {
        #[serde_as(as = "DisplayFromStr")]
        tag_name: u64,
    }

    let ver: GithubRelease = get_github_client(&llc_config)
        .await?
        .request(Request::get(format!(
            "repos/{}/{}/releases/latest",
            llc_config.github().owner(),
            llc_config.github().repo()
        )))
        .map(|r| r.and_then(|res| res.with_successful_status()))
        .and_then(|res| res.json())
        .await?;
    info!("get version from GitHub API");
    Ok(ver.tag_name)
}

#[instrument(skip(llc_config), level = "trace")]
#[inline]
async fn get_version_zeroasso(llc_config: &LLCConfig) -> Result<u64, ZeroAssoApiError> {
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
    use smol_macros::test;

    test! {
    async fn test_get_version() {
        let llc_config = Arc::new(LLCConfig::default());
        let version = run(llc_config).await;
        assert!(version.is_ok());
        println!("[test_get_version] Version: {}", version.unwrap());
    }
        }

    test! {
    async fn test_get_from_github() {
        get_version_github(&LLCConfig::default()).await.unwrap();
    }
    }

    test! {
    async fn test_get_from_zeroasso() {
        get_version_zeroasso(&LLCConfig::default()).await.unwrap();
    }
        }
}
