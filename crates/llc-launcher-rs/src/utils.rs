use llc_rs::LLCConfig;
use nyquest::{AsyncClient, ClientBuilder};
use std::sync::LazyLock;
use tokio::{pin, select, sync::OnceCell};

#[derive(Debug, Copy, Clone)]
pub enum IconType {
    Error,
    Info,
}

#[cfg(target_os = "windows")]
pub fn create_msgbox(title: &str, content: &str, icon_type: IconType) {
    let icon_type = match icon_type {
        IconType::Error => msgbox::IconType::Error,
        IconType::Info => msgbox::IconType::Info,
    };
    if let Err(e) = msgbox::create(title, content, icon_type) {
        eprintln!("Failed to create message box: {e}");
    }
}

#[cfg(not(target_os = "windows"))]
pub fn create_msgbox(_title: &str, _content: &str, _icon_type: IconType) {}

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

pub fn default_client_builder() -> ClientBuilder {
    ClientBuilder::default()
        .user_agent(*USER_AGENT)
        .with_header("From", "ligh.tsing@gmail.com")
        .no_caching()
}

pub async fn client() -> nyquest::Result<&'static AsyncClient> {
    static CLIENT: OnceCell<AsyncClient> = OnceCell::const_new();
    CLIENT
        .get_or_try_init(|| default_client_builder().build_async())
        .await
}

pub async fn github_client(llc_config: &LLCConfig) -> nyquest::Result<&'static AsyncClient> {
    static CLIENT: OnceCell<AsyncClient> = OnceCell::const_new();
    CLIENT
        .get_or_try_init(|| {
            default_client_builder()
                .base_url(llc_config.github().api_url().to_string())
                .build_async()
        })
        .await
}

pub static USER_AGENT: LazyLock<&str> = LazyLock::new(|| {
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

#[cfg(test)]
mod tests {
    use super::USER_AGENT;
    #[test]
    fn test_user_agent() {
        println!("{}", *USER_AGENT)
    }
}
