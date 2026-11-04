//! Deterministic release policy over reviewed change declarations, not inferred code semantics.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeSet {
    pub schema_version: u32,
    pub base_version: String,
    pub changes: Vec<Change>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub id: String,
    pub kind: ChangeKind,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Fix,
    Feature,
    Deprecation,
    Breaking,
    Docs,
    Internal,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum Bump {
    None,
    Patch,
    Minor,
    Major,
}

impl ChangeKind {
    fn bump(self) -> Bump {
        match self {
            Self::Fix => Bump::Patch,
            Self::Feature | Self::Deprecation => Bump::Minor,
            Self::Breaking => Bump::Major,
            Self::Docs | Self::Internal => Bump::None,
        }
    }
}

fn stable_version(text: &str) -> Result<[u64; 3]> {
    let parts: Vec<_> = text.split('.').collect();
    ensure!(
        parts.len() == 3,
        "expected stable MAJOR.MINOR.PATCH; prereleases/build metadata are not supported by policy 1"
    );
    let mut result = [0; 3];
    for (index, part) in parts.into_iter().enumerate() {
        ensure!(
            !part.is_empty()
                && part.bytes().all(|b| b.is_ascii_digit())
                && (part == "0" || !part.starts_with('0')),
            "version components must be decimal integers without leading zeroes"
        );
        result[index] = part.parse()?;
    }
    ensure!(
        result[0] > 0,
        "policy 1 requires a stable public contract (major >= 1)"
    );
    Ok(result)
}

/// `target` turns the recommendation into an exact-version gate. It never publishes or edits files.
pub fn evaluate(set: &ChangeSet, target: Option<&str>) -> Result<Value> {
    ensure!(set.schema_version == 1, "unsupported change-set schema");
    ensure!(
        !set.changes.is_empty() && set.changes.len() <= 256,
        "expected 1..256 classified changes"
    );
    let mut version = stable_version(&set.base_version)?;
    if let Some(target) = target {
        stable_version(target)?;
    }
    let mut ids = BTreeSet::new();
    let mut bump = Bump::None;
    let mut classified = Vec::new();
    for change in &set.changes {
        ensure!(
            !change.id.is_empty()
                && change.id.len() <= 96
                && change
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            "invalid change ID"
        );
        ensure!(ids.insert(&change.id), "duplicate change ID");
        ensure!(
            !change.summary.trim().is_empty() && change.summary.len() <= 4096,
            "change summary must contain 1..4096 bytes"
        );
        bump = bump.max(change.kind.bump());
        classified.push(json!({"id":change.id,"kind":change.kind,"impact":change.kind.bump()}));
    }
    let proposed = if bump == Bump::None {
        None
    } else {
        let index = match bump {
            Bump::Major => 0,
            Bump::Minor => 1,
            Bump::Patch => 2,
            Bump::None => unreachable!(),
        };
        version[index] = version[index]
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("version component overflow"))?;
        version[index + 1..].fill(0);
        Some(format!("{}.{}.{}", version[0], version[1], version[2]))
    };
    let matches = target.is_none() || proposed.as_deref() == target;
    let reason = if !matches {
        "Target does not match the required version; no publication is authorized."
    } else if proposed.is_none() {
        "Documentation/internal-only changes do not require a package release."
    } else {
        "Highest declared compatibility impact determines the next version."
    };
    Ok(json!({
        "schema_version":1,"policy_version":1,"status":if matches {"passed"} else {"blocked"},
        "base_version":set.base_version,"required_bump":bump,"proposed_version":proposed,
        "proposed_tag":proposed.as_ref().map(|v| format!("v{v}")),"target_version":target,
        "release_required":proposed.is_some(),"changes":classified,"reason":reason,
        "basis":"reviewed change declarations; code compatibility and completeness require review",
        "mutation":false,"publication_authorized":false
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn set(base: &str, kinds: &[&str]) -> ChangeSet {
        serde_json::from_value(json!({"schema_version":1,"base_version":base,"changes":kinds.iter().enumerate().map(|(i,k)| json!({"id":format!("C-{i}"),"kind":k,"summary":"Reviewed change"})).collect::<Vec<_>>()})).unwrap()
    }
    #[test]
    fn highest_impact_wins_and_lower_components_reset() {
        for (kinds, expected) in [
            (vec!["fix", "docs"], "1.3.10"),
            (vec!["fix", "feature"], "1.4.0"),
            (vec!["deprecation"], "1.4.0"),
            (vec!["feature", "breaking", "fix"], "2.0.0"),
        ] {
            let result = evaluate(&set("1.3.9", &kinds), Some(expected)).unwrap();
            assert_eq!(result["status"], "passed");
            assert_eq!(result["proposed_version"], expected);
            assert_eq!(result["publication_authorized"], false);
        }
    }
    #[test]
    fn rejects_under_and_over_bumps_and_reusing_the_base() {
        for target in ["1.3.0", "1.3.2", "1.4.0", "2.0.0"] {
            assert_eq!(
                evaluate(&set("1.3.0", &["fix"]), Some(target)).unwrap()["status"],
                "blocked"
            );
        }
        assert_eq!(
            evaluate(&set("1.3.0", &["fix"]), Some("1.3.1")).unwrap()["status"],
            "passed"
        );
        assert_eq!(
            evaluate(&set("1.3.0", &["feature"]), Some("1.3.1")).unwrap()["status"],
            "blocked"
        );
    }
    #[test]
    fn documentation_and_internal_changes_do_not_create_a_release() {
        let s = set("1.3.0", &["docs", "internal"]);
        let result = evaluate(&s, None).unwrap();
        assert!(result["proposed_version"].is_null());
        assert_eq!(result["release_required"], false);
        assert_eq!(evaluate(&s, Some("1.3.1")).unwrap()["status"], "blocked");
    }
    #[test]
    fn ambiguous_or_invalid_contracts_and_overflow_fail_closed() {
        for base in [
            "v1.3.0",
            "01.3.0",
            "1.03.0",
            "1.3",
            "1.3.0-beta",
            "1.3.0+build",
            "0.3.0",
            "1.3.-1",
        ] {
            assert!(evaluate(&set(base, &["fix"]), None).is_err());
        }
        assert!(evaluate(&set("18446744073709551615.0.0", &["breaking"]), None).is_err());
        assert!(evaluate(&set("1.3.0", &[]), None).is_err());
        let mut s = set("1.3.0", &["fix", "feature"]);
        s.changes[1].id = s.changes[0].id.clone();
        assert!(evaluate(&s, None).is_err());
        s.changes.pop();
        s.schema_version = 2;
        assert!(evaluate(&s, None).is_err());
        assert!(
            serde_json::from_value::<ChangeSet>(
                json!({"schema_version":1,"base_version":"1.3.0","changes":[],"publish":true})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<Change>(json!({"id":"A","kind":"guess","summary":"x"}))
                .is_err()
        );
    }
}
