use crate::utils::USER_AGENT;
use reqwest::{
    Client, ClientBuilder,
    header::{self, HeaderMap, HeaderValue},
};
use std::sync::LazyLock;

pub mod download_file;
pub mod get_hash;
pub mod get_version;
mod utils;

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    ClientBuilder::default()
        .user_agent(*USER_AGENT)
        .default_headers(HeaderMap::from_iter([(
            header::FROM,
            HeaderValue::from_static("ligh.tsing@gmail.com"),
        )]))
        .brotli(true)
        .deflate(true)
        .gzip(true)
        .https_only(true)
        .build()
        .expect("Building HTTP client failed")
});
