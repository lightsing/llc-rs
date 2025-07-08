use futures_util::{AsyncReadExt, FutureExt, StreamExt, TryFutureExt, stream::FuturesUnordered};
use nyquest::{AsyncClient, Request};
use std::{fmt::Debug, future::ready};
use url::Url;

pub trait ClientExt {
    fn download(&self, url: Url) -> impl Future<Output = nyquest::Result<Vec<u8>>> + Send;

    fn get_json<I, T>(&self, urls: I) -> impl Future<Output = nyquest::Result<T>> + Send
    where
        I: Iterator<Item = Url> + ExactSizeIterator + Send,
        T: serde::de::DeserializeOwned + Send + 'static;
}

impl ClientExt for AsyncClient {
    fn download(&self, url: Url) -> impl Future<Output = nyquest::Result<Vec<u8>>> + Send {
        self.request(Request::get(url.to_string()))
            .map(|r| r.and_then(|res| res.with_successful_status()))
            .and_then(|res| async {
                let len = res.content_length().unwrap_or(10 * 1024 * 1024); // Default to 10MB if unknown
                let mut buffer = Vec::with_capacity(len as _);
                res.into_async_read()
                    .read_to_end(&mut buffer)
                    .await
                    .map_err(nyquest::Error::Io)
                    .map(|_| buffer)
            })
    }

    fn get_json<I, T>(&self, urls: I) -> impl Future<Output = nyquest::Result<T>> + Send
    where
        I: Iterator<Item = Url> + ExactSizeIterator + Send,
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let n_urls = urls.len();

        async move {
            let (_, res) = urls
                .enumerate()
                .map(move |(idx, url)| {
                    self.request(Request::get(url.to_string()))
                        .map(|r| r.and_then(|res| res.with_successful_status()))
                        .and_then(|res| res.json::<T>())
                        .map(move |res| (idx, res.map(|v| (url, v))))
                })
                .collect::<FuturesUnordered<_>>()
                .skip_while(move |(idx, res)| {
                    ready(
                        *idx != n_urls - 1 // keep trying until the last node
                            && res.as_ref().inspect_err(|e| error!("{e}")).is_err(),
                    )
                })
                .next()
                .await
                .infallible();

            let (url, res) = res?;
            info!("request JSON from {url}");
            Ok(res)
        }
    }
}

pub trait OptionExt<T> {
    fn infallible(self) -> T;
}

impl<T> OptionExt<T> for Option<T> {
    #[cfg(not(debug_assertions))]
    fn infallible(self) -> T {
        unsafe { self.unwrap_unchecked() }
    }

    #[cfg(debug_assertions)]
    fn infallible(self) -> T {
        self.expect("infallible")
    }
}

pub trait ResultExt<T, E> {
    fn infallible(self) -> T;
}

impl<T, E: Debug> ResultExt<T, E> for Result<T, E> {
    #[cfg(not(debug_assertions))]
    fn infallible(self) -> T {
        unsafe { self.unwrap_unchecked() }
    }

    #[cfg(debug_assertions)]
    fn infallible(self) -> T {
        self.expect("infallible")
    }
}
