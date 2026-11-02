use super::*;
use std::collections::BTreeSet;

pub(super) fn validate(r: &Request, root: &Path) -> Result<()> {
    ensure!(r.schema_version == 1, "unsupported workflow schema");
    ensure!(
        r.id.len() >= 5
            && r.id.len() <= 60
            && r.id.as_bytes()[..3].iter().all(u8::is_ascii_digit)
            && r.id.as_bytes()[3] == b'-'
            && r.id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
        "workflow ID must be NNN-lowercase-slug"
    );
    ensure!(
        !r.title.trim().is_empty() && r.title.len() <= 200 && !r.title.contains(['\n', '\r', '\0']),
        "one-line title required"
    );
    ensure!(
        !r.request.trim().is_empty() && r.request.len() <= 8192 && !r.request.contains('\0'),
        "request required, maximum 8192 bytes"
    );
    ensure!(
        ["feature", "bug", "dependencies", "pr"].contains(&r.recipe.as_str()),
        "unknown workflow recipe"
    );
    r.context.validate()?;
    ensure!(
        r.files.len() <= 20 && r.files.values().map(String::len).sum::<usize>() <= 32768,
        "regression files exceed bound"
    );
    for (name, content) in &r.files {
        ensure!(
            Path::new(name).file_name().and_then(|x| x.to_str()) == Some(name)
                && !name.starts_with('.')
                && !name.contains(['\\', '/'])
                && name.len() <= 100
                && !content.contains('\0'),
            "safe plain regression filename required"
        );
    }
    ensure!(
        !r.checks.is_empty() && r.checks.len() <= 20,
        "select at least one explicit verification command"
    );
    let mut ids = BTreeSet::new();
    for c in &r.checks {
        ensure!(
            valid_id(&c.id)
                && ids.insert(&c.id)
                && !c.argv.is_empty()
                && c.argv.len() <= 40
                && c.argv
                    .iter()
                    .all(|a| !a.is_empty() && a.len() <= 4096 && !a.contains('\0'))
                && c.timeout_seconds > 0
                && c.timeout_seconds <= 600,
            "invalid verification command/ID/limit"
        );
        ensure!(
            c.before_exit.is_none_or(|code| code > 0 && code <= 255),
            "baseline must require a nonzero expected exit"
        );
        if c.before_exit.is_some() {
            ensure!(
                !r.files.is_empty()
                    && c.argv.iter().any(|a| a.starts_with("{checks}/")
                        && r.files.contains_key(a.trim_start_matches("{checks}/"))),
                "bug baseline requires a frozen external regression file"
            );
        }
    }
    if r.recipe == "dependencies" {
        ensure!(
            r.checks.iter().any(|c| c.rescan),
            "dependency remediation requires a SARIF rescan check plus project tests"
        );
        ensure!(
            r.checks.iter().any(|c| !c.rescan),
            "dependency remediation also requires project tests"
        );
    }
    for c in r.checks.iter().filter(|c| c.rescan) {
        let executable = Path::new(&c.argv[0])
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        ensure!(
            ["osv-scanner", "semgrep"].contains(&executable) && c.before_exit.is_none(),
            "rescan must use OSV-Scanner or Semgrep"
        );
        ensure!(
            c.argv
                .iter()
                .any(|a| a == "sarif" || a == "--format=sarif" || a == "--sarif"),
            "rescan must output SARIF to stdout"
        );
        ensure!(
            executable != "semgrep" || c.argv.iter().any(|a| a == "--error"),
            "Semgrep rescan requires --error so findings cannot appear as a clean exit"
        );
    }
    ensure!(
        r.recipe != "bug" || r.checks.iter().any(|c| c.before_exit.is_some()),
        "bug recipe needs an external failing regression; add a check with before_exit"
    );
    let mut humans = BTreeSet::new();
    ensure!(
        r.human_gates
            .iter()
            .all(|id| valid_id(id) && humans.insert(id)),
        "invalid/duplicate human gate"
    );
    if let Some(p) = &r.proposal {
        ensure!(p.id == r.id, "proposal/workflow IDs differ");
        crate::planning::validate(p, root)?;
    }
    Ok(())
}
pub fn suggested(root: &Path) -> Result<Vec<Check>> {
    let mut checks = vec![];
    if root.join("Cargo.toml").is_file() {
        for (id, args) in [
            ("rust-format", vec!["cargo", "fmt", "--all", "--check"]),
            (
                "rust-tests",
                vec!["cargo", "test", "--workspace", "--locked"],
            ),
        ] {
            checks.push(Check {
                id: id.into(),
                argv: args.into_iter().map(str::to_owned).collect(),
                before_exit: None,
                rescan: false,
                task_ids: vec![],
                timeout_seconds: 600,
            });
        }
    } else if root.join("package.json").is_file() {
        let p: Value = read_json(&root.join("package.json"))?;
        let runner = if root.join("pnpm-lock.yaml").is_file() {
            "pnpm"
        } else if root.join("yarn.lock").is_file() {
            "yarn"
        } else if root.join("bun.lock").is_file() || root.join("bun.lockb").is_file() {
            "bun"
        } else {
            "npm"
        };
        for id in ["typecheck", "test", "build"] {
            if p["scripts"][id].as_str().is_some() {
                checks.push(Check {
                    id: format!("node-{id}"),
                    argv: vec![runner.into(), "run".into(), id.into()],
                    before_exit: None,
                    rescan: false,
                    task_ids: vec![],
                    timeout_seconds: 600,
                });
            }
        }
    }
    Ok(checks)
}
pub fn prepare(r: Request, root: &Path, ops: &Path, apply: bool) -> Result<Value> {
    ensure!(
        r.planning_operation.is_none(),
        "native planning receipt must be resolved by the owning Workbench/terminal"
    );
    prepare_with_receipt(r, root, ops, apply, None)
}
pub(super) fn prepare_with_receipt(
    mut r: Request,
    root: &Path,
    ops: &Path,
    apply: bool,
    planning_call: Option<forge_domain::usage::TokenCall>,
) -> Result<Value> {
    let root = guided::root(root)?;
    let ops = guided::external(ops, &root)?;
    let dir = directory(&root, &ops, &r.id)?;
    if r.checks.is_empty() {
        r.checks = suggested(&root)?;
    }
    validate(&r, &root)?;
    let binary = std::env::current_exe()?.canonicalize()?;
    let bytes = std::fs::read(&binary)?;
    ensure!(
        bytes.len() <= 256 * 1024 * 1024,
        "evaluator exceeds 256 MiB"
    );
    let binary_hash = digest(&bytes);
    let evaluator = ops.join("evaluators").join(format!("forge-{binary_hash}"));
    let mut scanner_inputs = BTreeMap::new();
    for check in r.checks.iter().filter(|c| c.rescan) {
        let raw = Path::new(&check.argv[0]);
        let path = if raw.is_absolute() {
            raw.to_path_buf()
        } else {
            forge_adapters::clients::find_executable(&check.argv[0])
                .context("selected scanner not installed; no automatic installation")?
        };
        let path = guided::external(&path.canonicalize()?, &root)?;
        let bytes = std::fs::read(&path)?;
        ensure!(
            bytes.len() <= 256 * 1024 * 1024,
            "scanner executable exceeds bound"
        );
        scanner_inputs.insert(path, digest(&bytes));
    }
    let contract = CheckContract {
        schema_version: 1,
        root: root.clone(),
        checks: r.checks.clone(),
        files: r.files.clone(),
        scanner_inputs: scanner_inputs.clone(),
    };
    let contract_text = serde_json::to_string_pretty(&contract)?;
    let mut inputs = scanner_inputs;
    for (name, text) in &r.files {
        inputs.insert(dir.join("regressions").join(name), digest(text.as_bytes()));
    }
    inputs.insert(evaluator.clone(), binary_hash.clone());
    inputs.insert(dir.join("checks.json"), digest(contract_text.as_bytes()));
    let gates = r
        .checks
        .iter()
        .map(|c| Gate {
            id: c.id.clone(),
            argv: vec![
                evaluator.to_string_lossy().into_owned(),
                "workflow".into(),
                "check".into(),
                dir.join("checks.json").to_string_lossy().into_owned(),
                "--id".into(),
                c.id.clone(),
                "--root".into(),
                "{repo}".into(),
            ],
            timeout_seconds: c.timeout_seconds + 5,
            inputs: inputs.clone(),
            cache: Default::default(),
        })
        .collect();
    let mut manifest = if let Some(p) = &r.proposal {
        crate::planning::validate(p, &root)?
    } else {
        guided::manifest(
            &root,
            &r.id,
            vec![guided::task(
                format!("{}-implement", r.id),
                r.request.clone(),
                vec![],
            )],
        )
    };
    manifest.context = r.context.clone();
    manifest.gates = gates;
    for task in &mut manifest.tasks {
        task.executor = Executor::Codex {
            permission: Permission::WorkspaceWrite,
            model: None,
        };
        task.timeout_seconds = 900;
        task.max_attempts = 1;
        // Replace only proposal scaffolding. Existing manifests/declared gates are never imported here.
        task.acceptance = r
            .checks
            .iter()
            .filter(|c| c.task_ids.is_empty() || c.task_ids.contains(&task.id))
            .map(|c| Acceptance::Gate { id: c.id.clone() })
            .collect();
        ensure!(
            !task.acceptance.is_empty(),
            "every task needs a matching external check"
        );
        task.description = format!(
            "{}\n{}\nRecipe: {}. Stay inside the reviewed request. Preserve unrelated files, configured model/account/permissions. Never commit, push, publish or install services. External checks determine completion.",
            task.description, r.request, r.recipe
        );
    }
    for check in &r.checks {
        ensure!(
            check
                .task_ids
                .iter()
                .all(|id| manifest.tasks.iter().any(|t| &t.id == id)),
            "check references an unknown task"
        );
    }
    if !r.human_gates.is_empty() {
        let mut acceptance = r
            .checks
            .iter()
            .map(|c| Acceptance::Gate { id: c.id.clone() })
            .collect::<Vec<_>>();
        acceptance.extend(
            r.human_gates
                .iter()
                .map(|id| Acceptance::Human { id: id.clone() }),
        );
        manifest.tasks.push(Task{id:format!("{}-accept",r.id),repo:"project".into(),description:"Final verification and explicitly declared human acceptance; scope consent never satisfies this task.".into(),dependencies:manifest.tasks.iter().map(|t|t.id.clone()).collect(),priority:100,executor:Executor::Command{argv:vec![evaluator.to_string_lossy().into_owned(),"workflow".into(),"check".into(),dir.join("checks.json").to_string_lossy().into_owned(),"--id".into(),r.checks[0].id.clone(),"--root".into(),root.to_string_lossy().into_owned()]},acceptance,timeout_seconds:900,max_attempts:1,context:None,budget:Budget::default()});
    }
    manifest.validate()?;
    let spec = root.join("specs").join(&r.id);
    let documents = BTreeMap::from([
        (
            spec.join("spec.md"),
            format!("# {}\n\n{}\n\nRecipe: {}\n", r.title, r.request, r.recipe),
        ),
        (
            spec.join("plan.md"),
            format!(
                "# Reviewed execution\n\n{}\n\nChecks and exact argv:\n```json\n{}\n```\n\nScope consent is not acceptance. Memory optional. No publication.\n",
                r.request,
                serde_json::to_string_pretty(&r.checks)?
            ),
        ),
        (
            spec.join("tasks.md"),
            manifest
                .tasks
                .iter()
                .map(|t| {
                    format!(
                        "- [ ] {} {}\n",
                        t.id,
                        t.description.lines().next().unwrap_or("")
                    )
                })
                .collect::<String>(),
        ),
    ]);
    for (path, body) in &documents {
        guided::check_file(path, body)?;
    }
    let c = Candidate {
        schema_version: 1,
        root: root.clone(),
        source: revision(&root)?,
        request: r,
        manifest,
        documents,
        checks: contract,
        evaluator: evaluator.clone(),
        evaluator_sha256: binary_hash,
        planning_call,
    };
    let scope = consent_digest(&c)?;
    if apply {
        let root_map = BTreeMap::from([("project".into(), root.clone())]);
        let _lock = WorktreeLocks::acquire(&root_map)?;
        ensure!(
            revision(&root)? == c.source,
            "source changed during preparation"
        );
        // Preflight all conflicts before any writes. An identical preparation is safe to repeat.
        let mut files = vec![
            (
                dir.join("candidate.json"),
                serde_json::to_string_pretty(&c)?,
            ),
            (dir.join("checks.json"), contract_text),
            (
                dir.join("release.json"),
                serde_json::to_string_pretty(&c.manifest)?,
            ),
        ];
        files.extend(
            c.request
                .files
                .iter()
                .map(|(name, text)| (dir.join("regressions").join(name), text.clone())),
        );
        for (path, text) in &files {
            guided::check_file(path, text)?;
        }
        if evaluator.exists() {
            ensure!(
                std::fs::read(&evaluator)? == bytes,
                "evaluator path conflict"
            );
        } else {
            guided::create(&evaluator, "")?;
            std::fs::write(&evaluator, bytes)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&evaluator, std::fs::Permissions::from_mode(0o700))?;
            }
        }
        for (path, text) in &files {
            guided::create(path, text)?;
        }
    }
    Ok(
        json!({"schema_version":1,"workflow_schema":1,"status":if apply{"prepared"}else{"preview"},"workflow_id":c.request.id,"scope_digest":scope,"candidate":c,"executed":false,"acceptance":"not_evaluated","native_inference_calls":0,"scope_warning":"Review exact commands and external regression source. Native permissions and declared human gates remain independent."}),
    )
}
