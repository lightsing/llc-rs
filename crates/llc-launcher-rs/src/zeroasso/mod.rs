use llc_rs::LLCConfig;
use nyquest::{AsyncClient, ClientBuilder, Result};
use std::sync::LazyLock;
use tokio::sync::OnceCell;

pub mod download_file;
pub mod get_hash;
pub mod get_version;
mod utils;

static USER_AGENT: LazyLock<&str> = LazyLock::new(|| {
    let os_info = os_info::get();
    let os_ty = os_info.os_type();
    Box::leak(format!(
        "{pkg}/{ver} ({os} {os_ver}; {rustc}; {arch}; +{homepage}) nyquest/0.2 (+https://github.com/bdbai/nyquest)",
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

fn default_client_builder() -> ClientBuilder {
    ClientBuilder::default()
        .user_agent(*USER_AGENT)
        .with_header("From", "ligh.tsing@gmail.com")
        .no_caching()
}

async fn github_client(llc_config: &LLCConfig) -> Result<&'static AsyncClient> {
    static CLIENT: OnceCell<AsyncClient> = OnceCell::const_new();
    CLIENT
        .get_or_try_init(|| {
            default_client_builder()
                .base_url(llc_config.github().api_url().to_string())
                .build_async()
        })
        .await
}

async fn client() -> Result<&'static AsyncClient> {
    static CLIENT: OnceCell<AsyncClient> = OnceCell::const_new();
    CLIENT
        .get_or_try_init(|| default_client_builder().build_async())
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_agent() {
        println!("{}", *USER_AGENT)
    }
}
