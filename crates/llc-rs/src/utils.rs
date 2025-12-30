use futures_util::{AsyncReadExt, TryFutureExt};
use nyquest::{AsyncClient, Request, r#async::Response};
use std::{fmt::Debug, path::Path};
use url::Url;

pub trait ClientExt {
    fn get<I>(&self, urls: I) -> impl Future<Output = nyquest::Result<Response>> + Send
    where
        I: Iterator<Item = Url> + Send;

    /// Download the content from one of the given URLs.
    fn download<I>(&self, urls: I) -> impl Future<Output = nyquest::Result<Vec<u8>>> + Send
    where
        I: Iterator<Item = Url> + Send;

    /// Download the content from one of the given URLs to the given destination path.
    fn download_to<I, P>(
        &self,
        urls: I,
        dest: P,
    ) -> impl Future<Output = nyquest::Result<()>> + Send
    where
        I: Iterator<Item = Url> + Send,
        P: AsRef<Path>;

    /// Get and deserialize JSON content from one of the given URLs.
    fn get_json<I, T>(&self, urls: I) -> impl Future<Output = nyquest::Result<T>> + Send
    where
        I: Iterator<Item = Url> + Send,
        T: serde::de::DeserializeOwned + Send + 'static;
}

impl ClientExt for AsyncClient {
    fn get<I>(&self, urls: I) -> impl Future<Output = nyquest::Result<Response>> + Send
    where
        I: Iterator<Item = Url> + Send,
    {
        async move {
            let mut last_err = None;
            for url in urls {
                match self.request(Request::get(url.to_string())).await {
                    Ok(res) => {
                        if let Ok(res) = res.with_successful_status() {
                            return Ok(res);
                        }
                    }
                    Err(e) => {
                        error!("{e}");
                        last_err = Some(e);
                    }
                }
            }
            Err(last_err.unwrap())
        }
    }

    fn download<I>(&self, urls: I) -> impl Future<Output = nyquest::Result<Vec<u8>>> + Send
    where
        I: Iterator<Item = Url> + Send,
    {
        self.get(urls).and_then(|res| async {
            let len = res.content_length().unwrap_or(10 * 1024 * 1024); // Default to 10MB if unknown
            let mut buffer = Vec::with_capacity(len as _);
            res.into_async_read()
                .read_to_end(&mut buffer)
                .await
                .map_err(nyquest::Error::Io)
                .map(|_| buffer)
        })
    }

    fn download_to<I, P>(
        &self,
        urls: I,
        dest: P,
    ) -> impl Future<Output = nyquest::Result<()>> + Send
    where
        I: Iterator<Item = Url> + Send,
        P: AsRef<Path>,
    {
        let dest = dest.as_ref().to_path_buf();
        self.get(urls).and_then(move |res| async move {
            let mut file = smol::fs::File::create(&dest)
                .await
                .map_err(nyquest::Error::Io)?;
            let mut stream = res.into_async_read();
            smol::io::copy(&mut stream, &mut file)
                .await
                .map_err(nyquest::Error::Io)?;
            Ok(())
        })
    }

    fn get_json<I, T>(&self, urls: I) -> impl Future<Output = nyquest::Result<T>> + Send
    where
        I: Iterator<Item = Url> + Send,
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        self.get(urls).and_then(|res| res.json::<T>())
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
