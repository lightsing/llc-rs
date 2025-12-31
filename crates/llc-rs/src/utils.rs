use bytes::Bytes;
use futures_util::{TryFutureExt, TryStreamExt};
use reqwest::Response;
use std::{fmt::Debug, path::Path};
use tokio::io::AsyncWriteExt;
use url::Url;

pub mod eyre_backtrace;

#[derive(Debug, thiserror::Error)]
pub enum ReqwestExtError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

pub trait ClientExt {
    fn try_get<I>(&self, urls: I) -> impl Future<Output = Result<Response, ReqwestExtError>> + Send
    where
        I: Iterator<Item = Url> + Send;

    /// Download the content from one of the given URLs.
    fn download<I>(&self, urls: I) -> impl Future<Output = Result<Bytes, ReqwestExtError>> + Send
    where
        I: Iterator<Item = Url> + Send;

    /// Download the content from one of the given URLs to the given destination path.
    fn download_to<I, P>(
        &self,
        urls: I,
        dest: P,
    ) -> impl Future<Output = Result<(), ReqwestExtError>> + Send
    where
        I: Iterator<Item = Url> + Send,
        P: AsRef<Path>;

    /// Get and deserialize JSON content from one of the given URLs.
    fn get_json<I, T>(&self, urls: I) -> impl Future<Output = Result<T, ReqwestExtError>> + Send
    where
        I: Iterator<Item = Url> + Send,
        T: serde::de::DeserializeOwned + Send + 'static;
}

impl ClientExt for reqwest::Client {
    fn try_get<I>(&self, urls: I) -> impl Future<Output = Result<Response, ReqwestExtError>> + Send
    where
        I: Iterator<Item = Url> + Send,
    {
        async move {
            let mut last_err = None;
            for url in urls {
                match self
                    .get(url)
                    .send()
                    .await
                    .and_then(|res| res.error_for_status())
                {
                    Ok(res) => return Ok(res),
                    Err(e) => {
                        last_err = Some(e);
                    }
                }
            }
            Err(last_err.unwrap().into())
        }
    }

    fn download<I>(&self, urls: I) -> impl Future<Output = Result<Bytes, ReqwestExtError>> + Send
    where
        I: Iterator<Item = Url> + Send,
    {
        self.try_get(urls)
            .and_then(|res| res.bytes().map_err(|e| e.into()))
    }

    fn download_to<I, P>(
        &self,
        urls: I,
        dest: P,
    ) -> impl Future<Output = Result<(), ReqwestExtError>> + Send
    where
        I: Iterator<Item = Url> + Send,
        P: AsRef<Path>,
    {
        let dest = dest.as_ref().to_path_buf();
        self.try_get(urls).and_then(move |res| async move {
            let mut file = tokio::fs::File::create(&dest).await?;
            let mut stream = res.bytes_stream();
            while let Some(chunk) = stream.try_next().await? {
                file.write_all(&chunk).await?;
            }
            Ok(())
        })
    }

    fn get_json<I, T>(&self, urls: I) -> impl Future<Output = Result<T, ReqwestExtError>> + Send
    where
        I: Iterator<Item = Url> + Send,
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        self.try_get(urls)
            .and_then(|res| res.json::<T>().map_err(|e| e.into()))
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
