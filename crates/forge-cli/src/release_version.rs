use anyhow::{Result, ensure};
use serde_json::Value;
use std::{fs::File, io::Read, path::Path};

pub fn inspect(path: &Path, target: Option<&str>) -> Result<Value> {
    const LIMIT: u64 = 1024 * 1024;
    ensure!(
        std::fs::metadata(path)?.is_file(),
        "change set must be a regular JSON file"
    );
    let file = File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "change set must be a regular JSON file"
    );
    let mut bytes = Vec::new();
    file.take(LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= LIMIT,
        "change set exceeds 1 MiB limit"
    );
    let set = serde_json::from_slice(&bytes)?;
    let mut report = forge_domain::release_version::evaluate(&set, target)?;
    report["change_set_sha256"] = forge_domain::digest(&bytes).into();
    Ok(report)
}
