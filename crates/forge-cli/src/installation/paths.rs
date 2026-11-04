//! Read-only PATH inspection. Never execute discovered candidates or infer their versions.
use serde_json::{Value, json};
use std::{collections::BTreeSet, ffi::OsStr, fs, path::Path};

pub(super) const LIMIT: usize = 128;

pub(super) fn inspect(
    current: &Path,
    path: Option<&OsStr>,
    cwd: &Path,
    invoked: Option<&OsStr>,
) -> Value {
    let invoked = invoked
        .and_then(|s| Path::new(s).file_name())
        .and_then(OsStr::to_str);
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
        let targets: BTreeSet<_> = candidates
            .iter()
            .filter(|r| r["eligible_by_file_mode"] == true)
            .filter_map(|r| r["resolved"].as_str())
            .collect();
        let same_command = invoked.is_some_and(|s| {
            #[cfg(windows)]
            {
                s.eq_ignore_ascii_case(&name)
            }
            #[cfg(not(windows))]
            {
                s == name
            }
        });
        let different_selection = same_command
            && selected
                .as_ref()
                .is_some_and(|r| r["path_matches_running"] == false);
        let warning = if different_selection {
            Some(
                "PATH selects a different executable than the running command; updating another location may not change the command you launch.",
            )
        } else if targets.len() > 1 {
            Some(
                "Multiple distinct executable paths found; inspect PATH order before updating. Their versions are unknown.",
            )
        } else {
            None
        };
        let incomplete =
            dirs.len() > LIMIT || selected.as_ref().is_some_and(|r| r["resolved"].is_null());
        let status = if warning.is_some() {
            "attention"
        } else if incomplete {
            "incomplete"
        } else if selected.is_none() {
            "not_found"
        } else {
            "resolved"
        };
        let next_action = match status {
            "incomplete" => {
                "Inspection is incomplete. Review the PATH directory limit and unresolved targets; absence or a unique installation cannot be confirmed. No PATH changes are needed solely from this result."
            }
            "not_found" => {
                "Add the intended installation directory to PATH, then reopen the command."
            }
            "attention" => {
                "Compare the selected path with your intended installation. Adjust PATH or explicitly replace an obsolete copy after preserving a backup, then reopen Forge. No files were changed."
            }
            _ => {
                "PATH resolves to one executable target. An already-open session keeps its running version until restarted."
            }
        };
        commands.push(
            json!({"command":name,"selected":selected,"distinct_targets":targets.len(),"coverage":if incomplete {"partial"} else {"complete"},"status":status,"next_action":next_action,"candidates":candidates,"warning":warning}),
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
    #[cfg(unix)]
    fn invoked_alias_detects_conflict_even_with_a_different_target_basename() {
        let t = tempfile::tempdir().unwrap();
        let old = t.path().join("old");
        fs::create_dir(&old).unwrap();
        executable(&old.join("forge"));
        let current = t.path().join("illumitech-forge");
        executable(&current);
        let alias = t.path().join("forge");
        std::os::unix::fs::symlink(&current, &alias).unwrap();
        let path = std::env::join_paths([old]).unwrap();
        let v = inspect(&current, Some(&path), t.path(), Some(alias.as_os_str()));
        assert_eq!(v["commands"][0]["status"], "attention");
        assert_eq!(v["commands"][0]["selected"]["path_matches_running"], false);
        assert!(
            v["commands"][0]["warning"]
                .as_str()
                .unwrap()
                .contains("running command")
        );
    }
    #[test]
    #[cfg(unix)]
    fn truncated_scan_cannot_claim_absence_or_uniqueness() {
        let t = tempfile::tempdir().unwrap();
        let empty = t.path().join("empty");
        let available = t.path().join("available");
        fs::create_dir(&empty).unwrap();
        fs::create_dir(&available).unwrap();
        executable(&available.join("forge"));
        for found_first in [false, true] {
            let mut dirs = vec![empty.clone(); LIMIT + 1];
            dirs[if found_first { 0 } else { LIMIT }] = available.clone();
            let path = std::env::join_paths(dirs).unwrap();
            let v = inspect(
                &available.join("forge"),
                Some(&path),
                t.path(),
                Some(OsStr::new("forge")),
            );
            assert_eq!(v["commands"][0]["status"], "incomplete");
            assert_eq!(v["commands"][0]["coverage"], "partial");
            assert_eq!(v["truncated"], true);
        }
    }
    #[cfg(unix)]
    fn executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        fs::write(path, "#!/bin/sh\nexit 99\n").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    #[test]
    #[cfg(unix)]
    fn symlinks_and_repeated_entries_are_not_competing_installations() {
        let t = tempfile::tempdir().unwrap();
        let a = t.path().join("a");
        let b = t.path().join("b");
        fs::create_dir(&a).unwrap();
        fs::create_dir(&b).unwrap();
        for name in ["forge", "illumitech-forge"] {
            executable(&b.join(name));
            std::os::unix::fs::symlink(b.join(name), a.join(name)).unwrap();
        }
        let path = std::env::join_paths([&a, &a, &b]).unwrap();
        let value = inspect(
            &b.join("forge"),
            Some(&path),
            t.path(),
            Some(OsStr::new("forge")),
        );
        for row in value["commands"].as_array().unwrap() {
            assert_eq!(row["status"], "resolved");
            assert_eq!(row["distinct_targets"], 1);
            assert!(row["warning"].is_null());
            assert!(row["selected"]["version"].is_null());
        }
        assert_eq!(value["candidate_execution"], false);
    }
    #[test]
    #[cfg(unix)]
    fn conflicting_installations_report_selection_without_guessing_versions() {
        let t = tempfile::tempdir().unwrap();
        let a = t.path().join("old");
        let b = t.path().join("new");
        fs::create_dir(&a).unwrap();
        fs::create_dir(&b).unwrap();
        executable(&a.join("forge"));
        executable(&b.join("forge"));
        let path = std::env::join_paths([&a, &b]).unwrap();
        let value = inspect(
            &b.join("forge"),
            Some(&path),
            t.path(),
            Some(OsStr::new("forge")),
        );
        let row = &value["commands"][0];
        assert_eq!(row["status"], "attention");
        assert_eq!(row["distinct_targets"], 2);
        assert_eq!(row["selected"]["path"], a.join("forge").to_str().unwrap());
        assert_eq!(row["selected"]["path_matches_running"], false);
        assert!(row["selected"]["version"].is_null());
        assert_eq!(value["commands"][1]["status"], "not_found");
    }
    #[test]
    #[cfg(unix)]
    fn relative_paths_skip_non_executable_files_and_broken_links() {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir(t.path().join("bin")).unwrap();
        fs::write(t.path().join("forge"), "not executable").unwrap();
        executable(&t.path().join("bin/forge"));
        std::os::unix::fs::symlink("missing", t.path().join("illumitech-forge")).unwrap();
        let path = std::env::join_paths(["", "bin"]).unwrap();
        let v = inspect(
            &t.path().join("bin/forge"),
            Some(&path),
            t.path(),
            Some(OsStr::new("forge")),
        );
        assert_eq!(v["commands"][0]["status"], "resolved");
        assert_eq!(v["commands"][0]["selected"]["path_index"], 1);
        assert_eq!(v["commands"][1]["status"], "not_found");
    }
    #[test]
    fn bounds_path_and_does_not_treat_directories_as_executables() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(
            temp.path()
                .join(format!("forge{}", std::env::consts::EXE_SUFFIX)),
        )
        .unwrap();
        let path = std::env::join_paths(std::iter::repeat_n(temp.path(), LIMIT + 1)).unwrap();
        let value = inspect(
            &temp.path().join("running"),
            Some(&path),
            temp.path(),
            Some(OsStr::new("forge")),
        );
        assert_eq!(value["truncated"], true);
        assert_eq!(value["directories_inspected"], LIMIT);
        assert!(value["commands"][0]["selected"].is_null());
        assert_eq!(
            value["commands"][0]["candidates"].as_array().unwrap().len(),
            1
        );
    }
}
