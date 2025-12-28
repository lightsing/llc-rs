use crate::{
    LLCConfig,
    utils::{ClientExt, OptionExt, ResultExt},
};
use nyquest::{AsyncClient, ClientBuilder};
use std::sync::{LazyLock, OnceLock};

pub mod download_file;
pub mod get_hash;
pub mod get_version;

#[derive(Debug, thiserror::Error)]
pub enum ZeroAssoApiError {
    #[error(transparent)]
    Http(#[from] nyquest::Error),
    #[error("downloaded file {file_name} hash mismatch")]
    HashMismatch {
        file_name: String,
        expected_hash: [u8; 32],
        actual_hash: [u8; 32],
    },
}

async fn get_client() -> Result<&'static AsyncClient, ZeroAssoApiError> {
    static CLIENT: OnceLock<AsyncClient> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }

    let client = ClientBuilder::default()
        .user_agent(*USER_AGENT)
        .with_header("FROM", "ligh.tsing@gmail.com")
        .no_caching()
        .build_async()
        .await
        .inspect_err(|e| error!("Failed to initialize client: {e}"))
        .map_err(ZeroAssoApiError::from)?;

    CLIENT.set(client).ok();
    Ok(CLIENT.get().infallible())
}

async fn get_github_client(
    llc_config: &LLCConfig,
) -> Result<&'static AsyncClient, ZeroAssoApiError> {
    static CLIENT: OnceLock<AsyncClient> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }

    let client = ClientBuilder::default()
        .user_agent(*USER_AGENT)
        .with_header("FROM", "ligh.tsing@gmail.com")
        .base_url(llc_config.github().api_url().to_string())
        .no_caching()
        .build_async()
        .await
        .inspect_err(|e| error!("Failed to initialize client: {e}"))
        .map_err(ZeroAssoApiError::from)?;

    CLIENT.set(client).ok();
    Ok(CLIENT.get().infallible())
}

#[inline]
async fn request_zeroasso_api<T: serde::de::DeserializeOwned + Send + 'static>(
    llc_config: &LLCConfig,
    path: &str,
) -> Result<T, ZeroAssoApiError> {
    Ok(get_client()
        .await?
        .get_json(
            llc_config
                .api_nodes()
                .map(|base_url| base_url.join(path).infallible()),
        )
        .await?)
}

static USER_AGENT: LazyLock<&str> = LazyLock::new(|| {
    let os_info = os_info::get();
    let os_ty = os_info.os_type();
    Box::leak(format!(
        "Downloader/4.0.3 {pkg}/{ver} ({os} {os_ver}; {rustc}; {arch}; +{homepage}) nyquest/0.2 (+https://github.com/bdbai/nyquest)",
        pkg = env!("CARGO_PKG_NAME"),
        ver = env!("CARGO_PKG_VERSION"),
        os = match os_ty {
            os_info::Type::Windows => format_args!("Windows NT"),
            _ => format_args!("{os_ty}"),
        },
        os_ver = os_info.version(),
        rustc = env!("RUSTC_VERSION"),
        arch = std::env::consts::ARCH,
        homepage = env!("CARGO_PKG_HOMEPAGE"),
    ).into_boxed_str())
});

#[cfg(test)]
mod tests {
    use super::USER_AGENT;
    #[test]
    fn test_user_agent() {
        println!("{}", *USER_AGENT)
    }
}
