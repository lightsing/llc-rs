use crate::utils::client;
use eyre::Context;
use futures::{FutureExt, StreamExt, TryFutureExt, stream::FuturesUnordered};
use llc_rs::LLCConfig;
use nyquest::Request;
use std::future::ready;

#[instrument(skip(llc_config), level = "trace")]
#[inline]
pub async fn request_zeroasso_api<T: serde::de::DeserializeOwned>(
    llc_config: &LLCConfig,
    path: &str,
) -> eyre::Result<T> {
    let client = client()
        .await
        .inspect_err(|e| error!("Failed to create API client: {e}"))
        .context("无法创建 API 客户端。")?;

    let n_nodes = llc_config.api_nodes().count();
    let (_, res) = llc_config
        .api_nodes()
        .map(|base_url| base_url.join(path).expect("infallible"))
        .enumerate()
        .map(|(idx, url)| {
            client
                .request(Request::get(url.to_string()))
                .and_then(|fut| fut.json::<T>())
                .map(move |res| (idx, res.map(|v| (url, v))))
        })
        .collect::<FuturesUnordered<_>>()
        .skip_while(|(idx, res)| {
            ready(
                *idx != n_nodes - 1 // keep trying until the last node
                    && res.as_ref().inspect_err(|e| error!("{e}")).is_err(),
            )
        })
        .next()
        .await
        .expect("unreachable: should always yield at least one item");
    let (url, res) = res?;
    info!("request zeroasso API from {url}");
    Ok(res)
}
