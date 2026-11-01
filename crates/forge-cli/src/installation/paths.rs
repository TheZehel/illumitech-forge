//! Read-only PATH inspection. Never execute discovered candidates or infer their versions.
use serde_json::{Value, json};
use std::{collections::BTreeSet, ffi::OsStr, fs, path::Path};

pub(super) const LIMIT: usize = 128;

pub(super) fn inspect(current: &Path, path: Option<&OsStr>, cwd: &Path) -> Value {
    let dirs: Vec<_> = path
        .map(std::env::split_paths)
        .into_iter()
        .flatten()
        .collect();
    let current = current.canonicalize().ok();
    let mut commands = Vec::new();
    for base in ["forge", "illumitech-forge"] {
        let name = format!("{base}{}", std::env::consts::EXE_SUFFIX);
        let mut seen = BTreeSet::new();
        let mut candidates = Vec::new();
        let mut selected = None;
        for (index, dir) in dirs.iter().take(LIMIT).enumerate() {
            let dir = if dir.is_absolute() {
                dir.clone()
            } else {
                cwd.join(dir)
            };
            let file = dir.join(&name);
            if !seen.insert(file.clone()) {
                continue;
            }
            let Ok(meta) = fs::symlink_metadata(&file) else {
                continue;
            };
            let eligible = fs::metadata(&file).is_ok_and(|m| {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    m.is_file() && m.permissions().mode() & 0o111 != 0
                }
                #[cfg(not(unix))]
                {
                    m.is_file()
                }
            });
            let resolved = file.canonicalize().ok();
            let matches = resolved.as_ref().zip(current.as_ref()).map(|(a, b)| a == b);
            let row = json!({"path":file,"path_index":index,"symlink":meta.file_type().is_symlink(),
                "resolved":resolved,"eligible_by_file_mode":eligible,"path_matches_running":matches,
                "version":null,"provenance":"unknown; not executed"});
            if eligible && selected.is_none() {
                selected = Some(row.clone());
            }
            candidates.push(row);
        }
        let eligible_count = candidates
            .iter()
            .filter(|r| r["eligible_by_file_mode"] == true)
            .count();
        let warning = match selected.as_ref() {
            Some(row) if row["path_matches_running"] == false => Some(
                "PATH selects a different executable path than this running binary; updating another location may not change the command you launch.",
            ),
            _ if eligible_count > 1 => Some(
                "Multiple executable paths found; inspect their provenance and PATH order before updating.",
            ),
            _ => None,
        };
        commands.push(
            json!({"command":name,"selected":selected,"candidates":candidates,"warning":warning}),
        );
    }
    json!({"schema_version":1,"commands":commands,"directories_inspected":dirs.len().min(LIMIT),
        "directory_limit":LIMIT,"truncated":dirs.len()>LIMIT,"candidate_execution":false,
        "selection_basis":"PATH order, regular file and executable mode; ACLs, shell aliases/functions and command caches are not inspected. Candidate versions are unknown; version above describes only the running binary."})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_path_and_does_not_treat_directories_as_executables() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(
            temp.path()
                .join(format!("forge{}", std::env::consts::EXE_SUFFIX)),
        )
        .unwrap();
        let path = std::env::join_paths(std::iter::repeat_n(temp.path(), LIMIT + 1)).unwrap();
        let value = inspect(&temp.path().join("running"), Some(&path), temp.path());
        assert_eq!(value["truncated"], true);
        assert_eq!(value["directories_inspected"], LIMIT);
        assert!(value["commands"][0]["selected"].is_null());
        assert_eq!(
            value["commands"][0]["candidates"].as_array().unwrap().len(),
            1
        );
    }
}
