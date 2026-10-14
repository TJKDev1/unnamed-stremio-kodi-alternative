//! The only way the crate touches the network (D60). Everything else is plain code over data.

use std::future::Future;

/// A raw HTTP response, whatever the status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    /// The `Cache-Control` header, if any (fallback for body caching hints).
    pub cache_control: Option<String>,
    pub body: Vec<u8>,
}

/// The request never produced a response: DNS, TLS, connection or timeout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportError(pub String);

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TransportError {}

/// HTTP GET. Implementations apply their own timeout.
pub trait Transport: Send + Sync {
    fn get(&self, url: &str) -> impl Future<Output = Result<HttpResponse, TransportError>> + Send;
}

#[cfg(feature = "reqwest")]
pub use self::reqwest_impl::ReqwestTransport;

#[cfg(feature = "reqwest")]
mod reqwest_impl {
    use std::time::Duration;

    use super::{HttpResponse, Transport, TransportError};

    /// The default transport: reqwest + rustls, run on tokio by the caller (D60).
    #[derive(Debug, Clone)]
    pub struct ReqwestTransport {
        client: reqwest::Client,
    }

    impl ReqwestTransport {
        /// `timeout` bounds each request end to end, so one slow addon is cut off without
        /// affecting the others.
        pub fn new(timeout: Duration) -> Result<Self, TransportError> {
            let client = reqwest::Client::builder()
                .timeout(timeout)
                .user_agent(concat!("app-addons-stremio/", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|e| TransportError(e.to_string()))?;
            Ok(Self { client })
        }
    }

    impl Transport for ReqwestTransport {
        async fn get(&self, url: &str) -> Result<HttpResponse, TransportError> {
            // Errors print without the URL: addon URLs hold secrets.
            let err = |e: reqwest::Error| TransportError(e.without_url().to_string());
            let response = self.client.get(url).send().await.map_err(err)?;
            let status = response.status().as_u16();
            let cache_control = response
                .headers()
                .get(reqwest::header::CACHE_CONTROL)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            let body = response.bytes().await.map_err(err)?.to_vec();
            Ok(HttpResponse {
                status,
                cache_control,
                body,
            })
        }
    }
}
