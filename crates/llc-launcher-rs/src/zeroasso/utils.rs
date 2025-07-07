use crate::zeroasso::CLIENT;
use futures::{FutureExt, StreamExt, TryFutureExt, stream::FuturesUnordered};
use llc_rs::LLCConfig;
use std::future::ready;

#[instrument(skip(llc_config), level = "trace")]
#[inline]
pub async fn request_zeroasso_api<T: serde::de::DeserializeOwned>(
    llc_config: &LLCConfig,
    path: &str,
) -> eyre::Result<T> {
    let n_nodes = llc_config.api_nodes().count();
    let (_, res) = llc_config
        .api_nodes()
        .map(|base_url| base_url.join(path).expect("infallible"))
        .enumerate()
        .map(|(idx, url)| {
            CLIENT
                .get(url.clone())
                .send()
                .map(|r| r.and_then(|res| res.error_for_status()))
                .and_then(|res| res.json::<T>())
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
