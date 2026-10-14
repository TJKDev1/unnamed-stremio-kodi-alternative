//! Library storage, scanners and matchers.
//!
//! Placeholder crate from the repository layout (T0.1.1); real code lands in later tasks.

/// Crate name, so the placeholder has something to test.
pub const CRATE: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    #[test]
    fn crate_name() {
        assert_eq!(super::CRATE, "app-library");
    }
}
