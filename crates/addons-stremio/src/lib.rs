//! Stremio addon protocol client (spec: `docs/specs/stremio-client.md`).
//!
//! The crate is sans-IO (D60): models, validation, routes, routing and cache policy are plain
//! code over data, and all network access goes through a `Transport`.

mod de;
pub mod model;

pub use de::Dropped;
pub use model::*;
