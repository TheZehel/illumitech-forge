//! Explicit-prefix local updates. Caller hashes establish integrity, not publisher identity.
use anyhow::{Context, Result, ensure};
use clap::Subcommand;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

const PRODUCT: &str = "illumitech-forge";
const CURRENT: &str = ".forge-current.json";
const PENDING: &str = ".forge-update.json";
mod paths;
#[derive(Subcommand)]
pub enum Command {
    Doctor,
    Update {
        #[arg(long)]
        artifact: PathBuf,
        #[arg(long)]
        sha256: String,
        #[arg(long)]
        prefix: PathBuf,
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        allow_downgrade: bool,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema_version: u32,
    product: String,
    version: String,
    sha256: String,
    platform: String,
    source: String,
    provenance: String,
    previous: Option<String>,
    at_unix_ms: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    previous: Option<Receipt>,
    next: Receipt,
}
pub async fn dispatch(command: &Command) -> Result<Value> {
    match command {
        Command::Doctor => doctor(),
        Command::Update {
            artifact,
            sha256,
            prefix,
            apply,
            allow_downgrade,
            ..
        } => update(artifact, sha256, prefix, *apply, *allow_downgrade).await,
    }
}
pub fn host() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}
pub fn binary_name() -> &'static str {
    if cfg!(windows) {
        "illumitech-forge.exe"
    } else {
        PRODUCT
    }
}
/// Resolve without accepting symlinks (including dangling links), `..`, or special files.
/// Parents are operator-owned; the updater never grants protection against a malicious OS user
/// who can concurrently replace those parents. Such users already own this installation.
pub fn safe_path(path: &Path) -> Result<PathBuf> {
    ensure!(!path.as_os_str().is_empty(), "empty path");
    ensure!(
        !path.components().any(|c| matches!(c, Component::ParentDir)),
        "parent traversal refused"
    );
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut part = PathBuf::new();
    for c in absolute.components() {
        part.push(c);
        match fs::symlink_metadata(&part) {
            Ok(m) => ensure!(
                !m.file_type().is_symlink(),
                "symlink traversal refused: {}",
                part.display()
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
        #[cfg(windows)]
        if let Ok(m) = fs::symlink_metadata(&part) {
            use std::os::windows::fs::MetadataExt;
            ensure!(
                m.file_attributes() & 0x400 == 0,
                "reparse point traversal refused"
            );
        }
    }
    Ok(absolute)
}
pub fn hash(path: &Path) -> Result<String> {
    let path = safe_path(path)?;
    ensure!(fs::metadata(&path)?.is_file(), "regular file required");
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}
pub fn valid_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn read_json<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T> {
    let path = safe_path(path)?;
    let m = fs::metadata(&path)?;
    ensure!(
        m.is_file() && m.len() <= 4 * 1024 * 1024,
        "invalid or oversized metadata"
    );
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path.parent().context("metadata parent required")?;
    safe_path(path)?;
    let mut f = tempfile::NamedTempFile::new_in(parent)?;
    f.write_all(&serde_json::to_vec_pretty(value)?)?;
    f.as_file().sync_all()?;
    f.persist(path).map_err(|e| e.error)?;
    sync_dir(parent)?;
    Ok(())
}
pub fn sync_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
fn version(value: &str) -> Result<(u64, u64, u64)> {
    let numbers = value
        .split('.')
        .map(str::parse::<u64>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        numbers.len() == 3,
        "only stable three-component tool versions are supported"
    );
    Ok((numbers[0], numbers[1], numbers[2]))
}
fn native_artifact(path: &Path) -> Result<()> {
    let m = fs::metadata(safe_path(path)?)?;
    ensure!(
        m.is_file() && m.len() >= 64 && m.len() <= 512 * 1024 * 1024,
        "invalid executable artifact size/type"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            m.permissions().mode() & 0o111 != 0,
            "artifact is not executable"
        );
    }
    let mut b = [0u8; 64];
    File::open(path)?.read_exact(&mut b)?;
    let arch = std::env::consts::ARCH;
    let matches = match std::env::consts::OS {
        "macos" => {
            b[..4] == [0xcf, 0xfa, 0xed, 0xfe]
                && u32::from_le_bytes(b[4..8].try_into()?)
                    == match arch {
                        "aarch64" => 0x0100000c,
                        "x86_64" => 0x01000007,
                        _ => 0,
                    }
        }
        "linux" => {
            b[..4] == *b"\x7fELF"
                && b[4] == 2
                && b[5] == 1
                && u16::from_le_bytes(b[18..20].try_into()?)
                    == match arch {
                        "aarch64" => 183,
                        "x86_64" => 62,
                        _ => 0,
                    }
        }
        "windows" => {
            use std::io::{Seek, SeekFrom};
            let mut f = File::open(path)?;
            let offset = u32::from_le_bytes(b[60..64].try_into()?) as u64;
            ensure!(offset <= m.len().saturating_sub(24), "invalid PE header");
            f.seek(SeekFrom::Start(offset))?;
            let mut pe = [0; 24];
            f.read_exact(&mut pe)?;
            b[..2] == *b"MZ"
                && pe[..4] == *b"PE\0\0"
                && u16::from_le_bytes(pe[4..6].try_into()?)
                    == match arch {
                        "aarch64" => 0xaa64,
                        "x86_64" => 0x8664,
                        _ => 0,
                    }
        }
        _ => false,
    };
    ensure!(
        matches,
        "artifact target does not match host {} (thin native binaries required)",
        host()
    );
    Ok(())
}
async fn probe(path: &Path) -> Result<String> {
    native_artifact(path)?;
    let (_tx, rx) = tokio::sync::watch::channel(false);
    let out = forge_adapters::process::run(
        &[path.to_string_lossy().into_owned(), "--version".into()],
        path.parent().context("artifact parent required")?,
        5,
        rx,
    )
    .await?;
    ensure!(
        out.passed() && out.stdout.len() <= 256 && out.stderr.len() <= 256,
        "bounded executable version probe failed"
    );
    let text = out.stdout.trim();
    let v = text
        .strip_prefix("illumitech-forge ")
        .context("artifact is not a Forge executable")?;
    version(v)?;
    Ok(v.into())
}
fn archive(prefix: &Path, sha: &str) -> PathBuf {
    prefix.join(".forge-installations").join(sha)
}
fn audit(prefix: &Path, previous: &Option<Receipt>, next: &Receipt, recovery: &str) -> Result<()> {
    let directory = safe_path(&prefix.join(".forge-update-history"))?;
    fs::create_dir_all(&directory)?;
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).map_err(|e| anyhow::anyhow!("audit ID generation failed: {e}"))?;
    let id: String = random.iter().map(|b| format!("{b:02x}")).collect();
    write_json(
        &directory.join(format!("{}-{id}.json", crate::duration::now_ms())),
        &json!({"schema_version":1,"event":"binary_applied","previous":previous,"current":next,"recovery":recovery,"at_unix_ms":crate::duration::now_ms(),"state_migration":"none"}),
    )
}
fn verify_receipt(prefix: &Path, receipt: &Receipt) -> Result<()> {
    ensure!(
        receipt.schema_version == 1
            && receipt.product == PRODUCT
            && receipt.platform == host()
            && valid_hash(&receipt.sha256),
        "invalid installation receipt"
    );
    version(&receipt.version)?;
    let generation = archive(prefix, &receipt.sha256);
    let saved: Receipt = read_json(&generation.join("receipt.json"))?;
    ensure!(saved == *receipt, "installation receipt mismatch");
    native_artifact(&generation.join(binary_name()))?;
    ensure!(
        hash(&generation.join(binary_name()))? == receipt.sha256,
        "tampered verified binary/backup"
    );
    // Preserve and verify the entire rollback chain, including backups of equal-version builds.
    let mut next = receipt.previous.clone();
    let mut seen = std::collections::BTreeSet::from([receipt.sha256.clone()]);
    while let Some(sha) = next {
        ensure!(
            valid_hash(&sha) && seen.insert(sha.clone()) && seen.len() <= 128,
            "invalid backup chain"
        );
        let previous: Receipt = read_json(&archive(prefix, &sha).join("receipt.json"))?;
        ensure!(
            previous.sha256 == sha
                && previous.product == PRODUCT
                && previous.schema_version == 1
                && previous.platform == host(),
            "invalid backup receipt"
        );
        native_artifact(&archive(prefix, &sha).join(binary_name()))?;
        ensure!(
            hash(&archive(prefix, &sha).join(binary_name()))? == sha,
            "tampered backup"
        );
        next = previous.previous;
    }
    Ok(())
}
async fn update(
    artifact: &Path,
    sha: &str,
    prefix: &Path,
    apply: bool,
    downgrade: bool,
) -> Result<Value> {
    ensure!(
        valid_hash(sha),
        "sha256 must be 64 lowercase hexadecimal characters"
    );
    let artifact = safe_path(artifact)?;
    let prefix = safe_path(prefix)?;
    let target = safe_path(&prefix.join("bin").join(binary_name()))?;
    let current_path = safe_path(&prefix.join(CURRENT))?;
    let pending_path = safe_path(&prefix.join(PENDING))?;
    safe_path(&prefix.join(".forge-installations"))?;
    let history = safe_path(&prefix.join(".forge-update-history"))?;
    ensure!(
        !history.exists() || history.is_dir(),
        "invalid update audit directory"
    );
    native_artifact(&artifact)?;
    ensure!(
        hash(&artifact)? == sha,
        "artifact checksum mismatch; nothing installed"
    );
    // Probe a private verified copy so replacement of the caller's source cannot swap unchecked code.
    let staging = tempfile::tempdir()?;
    let staged = staging.path().canonicalize()?.join(binary_name());
    fs::copy(&artifact, &staged)?;
    ensure!(hash(&staged)? == sha, "artifact changed while staging");
    let reported = probe(&staged).await?;
    let mut lock = None;
    if apply {
        fs::create_dir_all(target.parent().unwrap())?;
        let path = safe_path(&prefix.join(".forge-update.lock"))?;
        ensure!(!path.exists() || path.is_file(), "invalid update lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        file.try_lock_exclusive()
            .context("another update owns this prefix")?;
        lock = Some(file);
    }
    let current: Option<Receipt> = if current_path.exists() {
        Some(read_json(&current_path)?)
    } else {
        None
    };
    if target.exists() {
        native_artifact(&target)?;
    }
    if let Some(c) = &current {
        verify_receipt(&prefix, c)?;
    }
    let pending: Option<Journal> = if pending_path.exists() {
        Some(read_json(&pending_path)?)
    } else {
        None
    };
    let mut effective = current.clone();
    let mut recovery = "none";
    if let Some(j) = &pending {
        ensure!(
            j.previous == current || current.as_ref() == Some(&j.next),
            "update journal/current mismatch"
        );
        verify_receipt(&prefix, &j.next)?;
        ensure!(
            j.next.sha256 == sha,
            "interrupted update requires the same verified artifact to recover"
        );
        let actual = if target.exists() {
            Some(hash(&target)?)
        } else {
            None
        };
        if actual.as_deref() == Some(&j.next.sha256) {
            effective = Some(j.next.clone());
            recovery = "finalize_receipt";
        } else {
            ensure!(
                actual.as_deref() == current.as_ref().map(|c| c.sha256.as_str()),
                "interrupted update target was changed"
            );
            recovery = "retry_atomic_swap";
        }
    } else {
        ensure!(
            target.exists() == current.is_some(),
            "unknown product or missing installed binary; refusing overwrite"
        );
        if let Some(c) = &current {
            ensure!(
                hash(&target)? == c.sha256,
                "installed binary differs from verified receipt"
            );
        }
    }
    if let Some(c) = &effective {
        ensure!(
            downgrade || version(&reported)? >= version(&c.version)?,
            "silent downgrade refused; restore matching state before explicit --allow-downgrade"
        );
    }
    // A rollback may select a prior generation; never create a cyclic backup chain.
    let archived = archive(&prefix, sha);
    let next = if archived.exists() {
        let r: Receipt = read_json(&archived.join("receipt.json"))?;
        verify_receipt(&prefix, &r)?;
        ensure!(
            r.sha256 == sha && r.version == reported,
            "artifact archive mismatch"
        );
        r
    } else {
        Receipt {
            schema_version: 1,
            product: PRODUCT.into(),
            version: reported.clone(),
            sha256: sha.into(),
            platform: host(),
            source: artifact.to_string_lossy().into_owned(),
            provenance: "caller-provided checksum; publisher identity/signature unknown".into(),
            previous: current.as_ref().map(|c| c.sha256.clone()),
            at_unix_ms: crate::duration::now_ms(),
        }
    };
    let unchanged = effective.as_ref().is_some_and(|c| c.sha256 == sha);
    let preview = json!({"schema_version":1,"status":if apply {if unchanged && recovery=="none" {"unchanged"}else{"applied"}} else {"preview"},"artifact":artifact,"sha256":sha,"platform":host(),"version":reported,"target":target,"current":current,"backup":current.as_ref().map(|c| archive(&prefix,&c.sha256).join(binary_name())),"receipt":current_path,"recovery":recovery,"allow_downgrade":downgrade,"provenance":"local caller checksum only; signing/publisher identity unknown","services":"unchanged"});
    if !apply {
        return Ok(preview);
    }
    if recovery == "finalize_receipt" {
        write_json(&current_path, &next)?;
        audit(
            &prefix,
            &pending.as_ref().unwrap().previous,
            &next,
            recovery,
        )?;
        fs::remove_file(&pending_path)?;
        sync_dir(&prefix)?;
        return Ok(preview);
    }
    if unchanged {
        return Ok(preview);
    }
    if !archived.exists() {
        fs::create_dir_all(archived.parent().unwrap())?;
        let generation = tempfile::tempdir_in(archived.parent().unwrap())?;
        fs::copy(&staged, generation.path().join(binary_name()))?;
        File::open(generation.path().join(binary_name()))?.sync_all()?;
        write_json(&generation.path().join("receipt.json"), &next)?;
        fs::rename(generation.path(), &archived)?;
        sync_dir(archived.parent().unwrap())?;
    }
    verify_receipt(&prefix, &next)?;
    write_json(
        &pending_path,
        &Journal {
            previous: current.clone(),
            next: next.clone(),
        },
    )?;
    let mut replacement = tempfile::NamedTempFile::new_in(target.parent().unwrap())?;
    std::io::copy(&mut File::open(&staged)?, &mut replacement)?;
    fs::set_permissions(replacement.path(), fs::metadata(&staged)?.permissions())?;
    replacement.as_file().sync_all()?;
    ensure!(hash(replacement.path())? == sha, "staged binary corrupted");
    replacement.persist(&target).map_err(|e| e.error)?;
    sync_dir(target.parent().unwrap())?;
    write_json(&current_path, &next)?;
    audit(
        &prefix,
        &pending
            .as_ref()
            .map_or_else(|| current.clone(), |p| p.previous.clone()),
        &next,
        recovery,
    )?;
    fs::remove_file(pending_path)?;
    sync_dir(&prefix)?;
    drop(lock);
    Ok(preview)
}
fn doctor() -> Result<Value> {
    let executable = std::env::current_exe()?;
    let mut candidates = Vec::new();
    let mut order = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        for (index, dir) in std::env::split_paths(&path).take(paths::LIMIT).enumerate() {
            let candidate = dir.join(binary_name());
            order.push(json!({"index":index,"directory":dir}));
            if let Ok(m) = fs::symlink_metadata(&candidate) {
                candidates.push(json!({"path":candidate,"path_index":index,"symlink":m.file_type().is_symlink(),"resolved":candidate.canonicalize().ok(),"regular_file":candidate.is_file(),"provenance":"unknown; not executed"}));
            }
        }
    }
    Ok(
        json!({"schema_version":1,"status":"diagnosed","current_executable":executable,"version":env!("CARGO_PKG_VERSION"),"platform":host(),"path_order":order,"competing_binaries":candidates,"path_diagnostics":paths::inspect(&executable, std::env::var_os("PATH").as_deref(), &std::env::current_dir()?),"provenance":"running binary; publisher unknown","mutation":false,"platform_capability":{"macos":"native host checks only; see validation report","linux":"implemented; native pilot required","windows":"experimental; native pilot required"}}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_versions_and_host_artifacts() {
        assert!(version("0.3.0").unwrap() < version("0.10.0").unwrap());
        assert!(version("0.3.0-beta").is_err());
        native_artifact(&std::env::current_exe().unwrap()).unwrap();
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("foreign");
        fs::write(&p, [0u8; 64]).unwrap();
        assert!(native_artifact(&p).is_err());
    }
}
