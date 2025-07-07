use crate::zeroasso::client;
use eyre::Context;
use futures::{FutureExt, StreamExt, TryFutureExt, stream::FuturesUnordered};
use llc_rs::LLCConfig;
use nyquest::Request;
use std::future::ready;
use tokio::{pin, select};

#[inline]
pub async fn select_ok2<F1, F2, T, E>(f1: F1, f2: F2) -> Result<T, E>
where
    F1: Future<Output = Result<T, E>>,
    F2: Future<Output = Result<T, E>>,
{
    pin!(f1);
    pin!(f2);

    select! {
        res1 = &mut f1 => match res1 {
            Ok(v) => Ok(v),
            Err(e1) => {
                match f2.await {
                    Ok(v) => Ok(v),
                    Err(_) => Err(e1),
                }
            }
        },
        res2 = &mut f2 => match res2 {
            Ok(v) => Ok(v),
            Err(e2) => {
                match f1.await {
                    Ok(v) => Ok(v),
                    Err(_) => Err(e2),
                }
            }
        },
    }
}

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
