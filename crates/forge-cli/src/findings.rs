//! Bounded, offline SARIF import. Findings and suggested fixes are untrusted data.
use anyhow::{Context, Result, ensure};
use forge_domain::digest;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};
fn decode_uri(text: &str) -> Result<String> {
    let mut bytes = vec![];
    let input = text.as_bytes();
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'%' {
            ensure!(i + 2 < input.len(), "incomplete URI escape");
            let digit = |b: u8| -> Result<u8> {
                match b {
                    b'0'..=b'9' => Ok(b - b'0'),
                    b'a'..=b'f' => Ok(b - b'a' + 10),
                    b'A'..=b'F' => Ok(b - b'A' + 10),
                    _ => anyhow::bail!("invalid URI escape"),
                }
            };
            let b = digit(input[i + 1])? * 16 + digit(input[i + 2])?;
            ensure!(b != b'/' && b != b'\\', "encoded path separator rejected");
            bytes.push(b);
            i += 3;
        } else {
            bytes.push(input[i]);
            i += 1;
        }
    }
    Ok(String::from_utf8(bytes)?)
}
fn relative_uri(uri: &str, root: &Path) -> Result<String> {
    let decoded = decode_uri(uri)?;
    let path = if let Some(local) = decoded.strip_prefix("file://") {
        ensure!(local.starts_with('/'), "remote file authorities rejected");
        let p = Path::new(local);
        ensure!(
            p.components().all(|c| !matches!(c, Component::ParentDir)),
            "URI traversal rejected"
        );
        p.strip_prefix(root)
            .context("file URI outside project")?
            .to_path_buf()
    } else {
        Path::new(decoded.trim_start_matches("./")).to_path_buf()
    };
    ensure!(
        !decoded.contains(['\\', '\0', '?', '#', '%'])
            && !path.is_absolute()
            && path.components().all(|c| matches!(c, Component::Normal(_)))
            && !path.as_os_str().is_empty()
            && !path.to_string_lossy().contains(':'),
        "unsafe SARIF artifact URI"
    );
    Ok(path.to_string_lossy().into_owned())
}
pub fn parse(v: &Value, root: &Path) -> Result<Value> {
    ensure!(v["version"] == "2.1.0", "SARIF 2.1.0 required");
    let runs = v["runs"].as_array().context("SARIF runs required")?;
    ensure!(!runs.is_empty() && runs.len() <= 20, "SARIF run limit");
    let mut rows = vec![];
    let mut seen = BTreeSet::new();
    let mut errors = vec![];
    for run in runs {
        ensure!(
            run.get("externalPropertyFileReferences").is_none(),
            "external SARIF references are not loaded"
        );
        let tool = run["tool"]["driver"]["name"]
            .as_str()
            .context("scanner identity required")?;
        if let Some(invocations) = run["invocations"].as_array() {
            for invocation in invocations {
                for field in [
                    "toolExecutionNotifications",
                    "toolConfigurationNotifications",
                ] {
                    if let Some(notifications) = invocation[field].as_array() {
                        for notification in notifications {
                            if notification["level"] == "error"
                                || notification["level"] == "warning"
                            {
                                errors.push(json!({"tool":tool,"reason":"scanner reported incomplete/error/warning coverage","notification":notification}));
                            }
                        }
                    }
                }
                if invocation["executionSuccessful"] == false {
                    errors.push(json!({"tool":tool,"reason":"scanner execution failed"}));
                }
            }
        }
        let artifacts = run["artifacts"].as_array();
        let results = run["results"]
            .as_array()
            .context("SARIF results required (empty list is valid)")?;
        ensure!(results.len() <= 10000, "SARIF finding limit");
        for result in results {
            let rule = result["ruleId"].as_str().context("ruleId required")?;
            let location = result["locations"]
                .as_array()
                .and_then(|x| x.first())
                .context("finding needs a source location")?;
            let physical = &location["physicalLocation"];
            let artifact = &physical["artifactLocation"];
            if let Some(base) = artifact["uriBaseId"].as_str() {
                if let Some(uri) = run["originalUriBaseIds"][base]["uri"].as_str() {
                    let decoded = decode_uri(uri)?;
                    ensure!(
                        decoded
                            .strip_prefix("file://")
                            .is_some_and(|p| Path::new(p) == root),
                        "SARIF URI base must equal the selected repository root"
                    );
                } else {
                    ensure!(base == "%SRCROOT%", "unresolved SARIF URI base");
                }
            }
            let uri = artifact["uri"]
                .as_str()
                .or_else(|| {
                    artifact["index"]
                        .as_u64()
                        .and_then(|n| artifacts?.get(n as usize)?["location"]["uri"].as_str())
                })
                .context("repo-relative artifact URI required")?;
            let uri = relative_uri(uri, root)?;
            let path = Path::new(&uri);
            let full = crate::guided::safe_path(&root.join(path))?;
            ensure!(full.starts_with(root), "finding outside project");
            let line = physical["region"]["startLine"].as_u64();
            let key = digest(serde_json::to_string(&(tool, rule, &uri, line))?.as_bytes());
            if seen.insert(key.clone()) {
                rows.push(json!({"id":key,"tool":tool,"tool_version":run["tool"]["driver"]["version"],"rule":rule,"path":uri,"line":line,"level":result["level"].as_str().unwrap_or("unknown"),"message":result["message"]["text"],"status":"unverified","suggested_fixes":"not_executed","severity_source":"scanner; no inferred CVSS or reachability"}));
            }
        }
    }
    Ok(
        json!({"schema_version":1,"findings_schema":1,"status":if errors.is_empty(){"imported"}else{"blocked"},"findings":rows,"scanner_errors":errors,"input_sha256":digest(&serde_json::to_vec(v)?),"resolved":false,"uri_base_binding":"Missing %SRCROOT% binds to selected repository; source revision must still be verified","mutation":false,"native_inference_calls":0,"next_action":"Select findings, review a dependency/bug workflow and require a successful rescan plus project tests. Disappearance alone is not proof of resolution."}),
    )
}
pub fn file(path: &Path, root: &Path) -> Result<Value> {
    let root = crate::guided::root(root)?;
    parse(&crate::workflow::read_json(path)?, &root)
}
#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn sarif_for_base(uri: &str) -> Value {
        json!({"version":"2.1.0","runs":[{"tool":{"driver":{"name":"OSV-Scanner"}},"results":[{"ruleId":"fixture","message":{"text":"Untrusted suggestion"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":uri},"region":{"startLine":1}}}]}]}]})
    }
    #[test]
    fn unsafe_locations_and_external_refs_fail() {
        let t = tempfile::tempdir().unwrap();
        for uri in [
            "../secret",
            "https://example.com/code",
            "/tmp/code",
            "%2e%2e/code",
            "a\\b",
        ] {
            assert!(parse(&sarif_for_base(uri), t.path()).is_err());
        }
        let mut s = sarif_for_base("Cargo.lock");
        s["runs"][0]["externalPropertyFileReferences"] = json!({});
        assert!(parse(&s, t.path()).is_err());
    }
    #[test]
    fn dedup_and_scanner_errors_do_not_resolve_findings() {
        let t = tempfile::tempdir().unwrap();
        let mut s = sarif_for_base("Cargo.lock");
        let row = s["runs"][0]["results"][0].clone();
        s["runs"][0]["results"].as_array_mut().unwrap().push(row);
        assert_eq!(
            parse(&s, t.path()).unwrap()["findings"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        s["runs"][0]["invocations"] = json!([{"executionSuccessful":false}]);
        assert_eq!(parse(&s, t.path()).unwrap()["status"], "blocked");
    }
}

#[cfg(test)]
mod uri_tests {
    use super::*;
    #[test]
    fn local_absolute_and_encoded_space_are_normalized() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path();
        assert_eq!(
            relative_uri("src/file%20name.rs", root).unwrap(),
            "src/file name.rs"
        );
        assert_eq!(
            relative_uri(&format!("file://{}/src/a.rs", root.display()), root).unwrap(),
            "src/a.rs"
        );
        for value in [
            "file://evil/tmp/code",
            "file:///other/code",
            "src%2fsecret",
            "%252e%252e/private",
        ] {
            assert!(relative_uri(value, root).is_err(), "{value}");
        }
    }
}

#[cfg(test)]
mod semgrep_base_tests {
    use super::*;
    #[test]
    fn source_root_placeholder_is_bound_to_selected_repo_but_foreign_bases_fail() {
        let t = tempfile::tempdir().unwrap();
        let mut value = super::tests::sarif_for_base("package.json");
        value["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]["uriBaseId"] =
            json!("%SRCROOT%");
        assert_eq!(
            parse(&value, t.path()).unwrap()["findings"][0]["path"],
            "package.json"
        );
        value["runs"][0]["originalUriBaseIds"] =
            json!({"%SRCROOT%":{"uri":"file:///foreign/project/"}});
        assert!(parse(&value, t.path()).is_err());
    }
}
