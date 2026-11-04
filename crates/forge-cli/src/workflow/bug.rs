//! Guided bug intake produces the existing frozen workflow contract.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Regression {
    pub expected: String,
    pub reproduction: String,
    pub filename: String,
    pub content: String,
    pub argv: Vec<String>,
    #[serde(default)]
    pub planned_files: Vec<String>,
}
pub fn request(
    p: crate::planning::Proposal,
    description: &str,
    operation: Option<&str>,
) -> Result<Request> {
    let bug = p
        .bug
        .as_ref()
        .context("Expected behavior or reproduction missing; clarify the bug before preparing")?;
    ensure!(
        !bug.expected.trim().is_empty() && !bug.reproduction.trim().is_empty(),
        "expected behavior and reproduction required"
    );
    ensure!(
        bug.planned_files.len() <= 100
            && bug.planned_files.iter().all(|name| !name.is_empty()
                && name.len() <= 4096
                && !Path::new(name).is_absolute()
                && Path::new(name)
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_)))),
        "planned source files must be bounded relative paths"
    );
    let mut checks = p
        .tasks
        .iter()
        .flat_map(|t| {
            t.acceptance.iter().map(|a| Check {
                id: a.id.clone(),
                argv: a.command.clone(),
                before_exit: None,
                rescan: false,
                task_ids: vec![t.id.clone()],
                timeout_seconds: 600,
            })
        })
        .collect::<Vec<_>>();
    checks.push(Check {
        id: "bug-regression".into(),
        argv: bug.argv.clone(),
        before_exit: Some(42),
        rescan: false,
        task_ids: vec![],
        timeout_seconds: 600,
    });
    Ok(Request {
        schema_version: 1,
        id: p.id.clone(),
        title: p.title.clone(),
        request: format!(
            "{description}\nExpected: {}\nReproduction: {}",
            bug.expected, bug.reproduction
        ),
        recipe: "bug".into(),
        checks,
        files: BTreeMap::from([(bug.filename.clone(), bug.content.clone())]),
        proposal: Some(p),
        context: context_default(),
        human_gates: vec![],
        planning_operation: operation.map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guided_request_requires_reproduction_and_retains_external_source() {
        let mut p:crate::planning::Proposal=serde_json::from_value(json!({"schema_version":1,"id":"058-bug","title":"Fix","summary":"Fix bug","tasks":[{"id":"fix","title":"Fix","dependencies":[],"acceptance":[{"id":"tests","description":"tests","command":["cargo","test"]}]}]})).unwrap();
        assert!(request(p.clone(), "bug", None).is_err());
        p.bug = Some(Regression {
            planned_files: vec!["lib.rs".into()],
            expected: "2".into(),
            reproduction: "add(1,1)".into(),
            filename: "verify.py".into(),
            content: "assert True".into(),
            argv: vec![
                "python3".into(),
                "{checks}/verify.py".into(),
                "{repo}".into(),
            ],
        });
        let r = request(p, "bug", Some("attempt-1")).unwrap();
        assert_eq!(r.recipe, "bug");
        assert_eq!(r.checks.last().unwrap().before_exit, Some(42));
        assert_eq!(r.files["verify.py"], "assert True");
    }
}
