//! Stremio addon protocol client (spec: `docs/specs/stremio-client.md`).
//!
//! The crate is sans-IO (D60): models, validation, routes, routing and cache policy are plain
//! code over data, and all network access goes through [`Transport`].

mod de;
pub mod manifest;
pub mod model;
pub mod route;
pub mod transport;

pub use de::Dropped;
pub use manifest::{Validated, Warning, fetch_manifest};
pub use model::*;
pub use route::ResourcePath;
pub use transport::{HttpResponse, Transport, TransportError};

/// Every addon transport URL ends with this.
pub const MANIFEST_PATH: &str = "/manifest.json";

/// Why a request to one addon failed. Failures are per addon: others still answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// No response at all (network, TLS, timeout).
    Transport(TransportError),
    /// The addon answered with a non-2xx status.
    Status(u16),
    /// The body wasn't a usable response.
    Parse(ParseError),
    /// The manifest is unusable.
    Manifest(String),
    /// The addon URL isn't a Stremio transport URL.
    InvalidUrl(String),
    /// A transport we deliberately don't speak (legacy `/stremio/v1`, `ipfs://`).
    Unsupported(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "network: {e}"),
            Self::Status(s) => write!(f, "HTTP {s}"),
            Self::Parse(e) => write!(f, "bad response: {e}"),
            Self::Manifest(e) => write!(f, "bad manifest: {e}"),
            Self::InvalidUrl(e) => write!(f, "invalid addon URL: {e}"),
            Self::Unsupported(e) => write!(f, "unsupported: {e}"),
        }
    }
}

impl std::error::Error for Error {}
