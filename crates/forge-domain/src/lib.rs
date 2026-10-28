//! Versioned contracts, fingerprints and durable orchestration state.
pub mod manifest;
pub mod revision;
pub mod state;
pub use manifest::*;
pub use revision::*;
pub use state::*;

pub fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
