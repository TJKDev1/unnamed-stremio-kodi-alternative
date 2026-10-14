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
