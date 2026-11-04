//! Versioned contracts, fingerprints and durable orchestration state.
pub mod duration;
pub mod manifest;
pub mod release_version;
pub mod revision;
pub mod state;
pub mod usage;
mod usage_store;
pub use manifest::*;
pub use revision::*;
pub use state::*;
pub use usage_store::unavailable as unavailable_token_usage;

pub fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
