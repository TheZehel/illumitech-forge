//! Explicit terminal workflow commands; chat messages never execute a release implicitly.
use super::*;
use crate::workflow;
use anyhow::Context as _;

pub(super) fn pending(app: &Arc<App>, chat: &Chat, text: &str) -> Result<Option<Pending>> {
    let command = text.split_whitespace().next().unwrap_or("").to_owned();
    if ![
        "/doctor",
        "/bug",
        "/plan",
        "/run",
        "/tasks",
        "/report",
        "/qa",
        "/qa-run",
        "/qa-status",
    ]
    .contains(&command.as_str())
    {
        return Ok(None);
    }
    let args = text
        .strip_prefix(command.as_str())
        .unwrap_or("")
        .trim()
        .to_owned();
    let current = chat.workflow_id.clone();
    let digest = chat.workflow_digest.clone();
    let mode = chat.mode;
    let qa_id = chat.qa_id.clone();
    let qa_digest = chat.qa_digest.clone();
    let (qa_cancel, cancel_rx) = tokio::sync::watch::channel(false);
    let qa_cancel = (command == "/qa-run").then_some(qa_cancel);
    let app = Arc::clone(app);
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce).map_err(|e| anyhow::anyhow!("operation ID: {e}"))?;
    let operation_id = format!("terminal-workflow-{}", forge_domain::digest(&nonce));
    let route = format!("workflow{command}");
    let handle = tokio::spawn(async move {
        if command == "/doctor" {
            return crate::workflow::doctor(&app.root).await;
        }
        if command == "/qa-run" {
            ensure!(
                !qa_id.is_empty() && !qa_digest.is_empty() && args == qa_digest,
                "Prepare /qa REQUEST e confirme /qa-run DIGEST do escopo QA revisado; aprovação Feature não autoriza QA"
            );
            return crate::modules::qa::start(
                &app.root,
                &app.workflow_ops()?,
                &qa_id,
                &args,
                cancel_rx,
            )
            .await;
        }
        if command == "/qa-status" {
            let id = if args.is_empty() { qa_id } else { args };
            ensure!(!id.is_empty(), "Use /qa-status ID ou prepare /qa REQUEST");
            return crate::modules::qa::status(&app.root, &app.workflow_ops()?, &id);
        }
        if command == "/qa" {
            ensure!(
                !args.is_empty(),
                "Use /qa REQUEST.json ou /qa light|standard|intensive MANIFEST"
            );
            let legacy = args.split_once(' ').and_then(|(level, path)| {
                let level = match level {
                    "light" => crate::modules::QaLevel::Light,
                    "standard" => crate::modules::QaLevel::Standard,
                    "intensive" => crate::modules::QaLevel::Intensive,
                    _ => return None,
                };
                Some((level, PathBuf::from(path.trim().trim_start_matches('@'))))
            });
            let (level, manifest) = if let Some(legacy) = legacy {
                legacy
            } else {
                let path = PathBuf::from(args.trim_start_matches('@'));
                if let Ok(value) = workflow::read_json::<Value>(&path)
                    && value.get("release_id").is_none()
                {
                    let request = serde_json::from_value(value)?;
                    return crate::modules::qa::prepare(
                        request,
                        &app.root,
                        &app.workflow_ops()?,
                        true,
                    );
                }
                // Bare legacy JSON/YAML manifests remain planning-only. Invalid QA
                // requests cannot be admitted as manifests by Context::load.
                (crate::modules::QaLevel::Standard, path)
            };
            let state = app.state.join("qa-plan-state");
            let context = crate::engine::Context::load(&manifest, &state)?;
            ensure!(
                context.roots.values().all(|root| root == &app.root),
                "O manifest QA deve pertencer ao projeto selecionado"
            );
            return crate::modules::dispatch(
                &crate::modules::Command::QaPlan { manifest, level },
                &state,
            )
            .await;
        }
        if command == "/plan" || command == "/bug" {
            ensure!(
                !args.is_empty(),
                "Use /plan PEDIDO ou /plan @ARQUIVO.json para uma regressão/receita especializada"
            );
            let request: workflow::Request = if let Some(path) = args.strip_prefix('@') {
                workflow::read_json(Path::new(path))?
            } else {
                let a: Action = serde_json::from_value(
                    json!({"operation_id":format!("{operation_id}-proposal"),"request":if command == "/bug" {format!("Corrigir um bug com regressão externa. {args}")}else{args.clone()}}),
                )?;
                let value = app.action("planning-propose", a).await?;
                let proposal: crate::planning::Proposal =
                    serde_json::from_value(value["proposal"].clone())
                        .context("native structured proposal required")?;
                if command == "/bug" {
                    workflow::bug::request(proposal, &args, value["operation_id"].as_str())?
                } else {
                    let checks=proposal.tasks.iter().flat_map(|t|t.acceptance.iter().map(|a|json!({"id":a.id,"argv":a.command,"timeout_seconds":600,"task_ids":[t.id]}))).collect::<Vec<_>>();
                    serde_json::from_value(
                        json!({"schema_version":1,"id":proposal.id,"title":proposal.title,"request":args,"recipe":"feature","checks":checks,"proposal":proposal,"planning_operation":value["operation_id"]}),
                    )?
                }
            };
            if command == "/plan" && mode == modes::Mode::Feature {
                crate::modules::feature::validate(&request, &app.root)?;
            }
            let a = serde_json::from_value(json!({"operation_id":operation_id,"request":request}))?;
            return app.workflow_action("workflow-prepare", a).await;
        }
        if command == "/tasks" && args.is_empty() {
            return workflow::list(&app.root, &app.workflow_ops()?);
        }
        if command == "/tasks" {
            let parts = args.split_whitespace().collect::<Vec<_>>();
            if parts.len() == 2 && ["resume", "pause", "cancel"].contains(&parts[0]) {
                let a = serde_json::from_value(json!({"operation_id":operation_id,"id":parts[1]}))?;
                return app
                    .workflow_action(&format!("workflow-{}", parts[0]), a)
                    .await;
            }
            if parts.first() == Some(&"approve") {
                ensure!(
                    parts.len() >= 6,
                    "/tasks approve ID TASK GATE FINGERPRINT REVIEWER"
                );
                let a = serde_json::from_value(
                    json!({"operation_id":operation_id,"id":parts[1],"task":parts[2],"gate":parts[3],"fingerprint":parts[4],"reviewer":parts[5..].join(" ")}),
                )?;
                return app.workflow_action("workflow-approve", a).await;
            }
        }
        if command == "/run" {
            ensure!(
                !current.is_empty() && !digest.is_empty(),
                "Use /plan primeiro e revise arquivos/comandos no painel"
            );
            ensure!(
                args == digest,
                "Confirmação explícita necessária: /run DIGEST exibido no plano. Isso não aprova a conclusão."
            );
            let a = serde_json::from_value(
                json!({"operation_id":operation_id,"id":current,"scope_digest":digest}),
            )?;
            return app.workflow_action("workflow-start", a).await;
        }
        let id = if args.is_empty() { current } else { args };
        ensure!(!id.is_empty(), "Use /tasks ID ou /report ID");
        let a = serde_json::from_value(json!({"operation_id":operation_id,"id":id}))?;
        let mut report = app.workflow_action("workflow-status", a).await?;
        if command == "/report" {
            let review = serde_json::from_value(
                json!({"operation_id":format!("{operation_id}-diff"),"id":id}),
            )?;
            report["local_review"] = app.workflow_action("workflow-review", review).await?;
        }
        Ok(report)
    });
    Ok(Some(Pending {
        handle,
        route,
        draft: String::new(),
        qa_cancel,
    }))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn qa_terminal_prepares_reviews_runs_and_reports_without_native_calls() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("project");
        std::fs::create_dir(&root).unwrap();
        forge_domain::git(&root, &["init", "-q"]).unwrap();
        forge_domain::git(
            &root,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "--allow-empty",
                "-qm",
                "fixture",
            ],
        )
        .unwrap();
        let script = base.join("check.sh");
        std::fs::write(&script, "printf observed > \"$1/../observed\"\nsleep 1\n").unwrap();
        let request = base.join("request.json");
        std::fs::write(&request, serde_json::to_vec(&json!({
            "schema_version":1,"id":"terminal-qa","title":"Terminal fixture QA","level":"light",
            "checks":[{"id":"quality","coverage_id":"QA-QUALITY","argv":["sh",script,"{repo}"],"timeout_seconds":5,"inputs":{script.to_str().unwrap():forge_domain::digest(&std::fs::read(&script).unwrap())}}]
        })).unwrap()).unwrap();
        // Any accidental native call fails: no client is launched by these QA routes.
        let app = App::new(
            &root,
            &base.join("state"),
            Some(base.join("nonexistent-native-client")),
            None,
        )
        .unwrap();
        let mut chat = Chat {
            mode: modes::Mode::Qa,
            workflow_id: "old-feature".into(),
            workflow_digest: "feature-consent".into(),
            ..Default::default()
        };
        let operation = pending(&app, &chat, &chat.mode.route(request.to_str().unwrap()))
            .unwrap()
            .unwrap();
        let prepared = operation.handle.await.unwrap().unwrap();
        assert_eq!(prepared["status"], "prepared");
        assert!(!base.join("observed").exists());
        chat.workflow_result(prepared);
        assert!(chat.workflow_id.is_empty() && chat.workflow_digest.is_empty());
        let command = chat.reviewed_run_command().unwrap();
        assert!(command.starts_with("/qa-run "));
        assert!(!base.join("observed").exists()); // Preparing the editor does not execute.
        assert!(
            pending(&app, &chat, "/run feature-consent")
                .unwrap()
                .unwrap()
                .handle
                .await
                .unwrap()
                .is_err()
        );
        assert!(
            pending(&app, &chat, "/qa-run wrong")
                .unwrap()
                .unwrap()
                .handle
                .await
                .unwrap()
                .is_err()
        );
        assert!(!base.join("observed").exists());
        let operation = pending(&app, &chat, &command).unwrap().unwrap();
        assert!(!operation.handle.is_finished());
        // Rendering remains independent while the asynchronous check is pending.
        chat.panel = Some(Panel::Workflow);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        terminal
            .draw(|frame| chat.draw(frame, &View::default(), &root))
            .unwrap();
        let rendered = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(rendered.contains("/qa-run"), "{rendered}");
        assert!(rendered.contains("QA-QUALITY"), "{rendered}");
        let result = operation.handle.await.unwrap().unwrap();
        assert_eq!(result["status"], "degraded");
        assert_eq!(result["checks"][0]["status"], "passed");
        assert!(base.join("observed").exists());
        chat.workflow_result(result);
        assert!(chat.qa_digest.is_empty());
        assert!(chat.reviewed_run_command().is_none());
        let status = pending(&app, &chat, "/qa-status")
            .unwrap()
            .unwrap()
            .handle
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status["status"], "degraded");
        assert_eq!(status["mutation"], false);
        let legacy_path = base.join("legacy.yaml");
        let legacy = json!({"schema_version":1,"release_id":"legacy-qa", "repos":{"project":{"path":root}},
            "tasks":[{"id":"review","repo":"project","description":"Legacy planning only", "executor":{"kind":"command","argv":["must-never-execute"]}, "acceptance":[{"kind":"human","id":"operator-review"}]}]});
        std::fs::write(&legacy_path, serde_yaml::to_string(&legacy).unwrap()).unwrap();
        for command in [
            format!("/qa {}", legacy_path.display()),
            format!("/qa standard {}", legacy_path.display()),
        ] {
            let value = pending(&app, &chat, &command)
                .unwrap()
                .unwrap()
                .handle
                .await
                .unwrap()
                .unwrap();
            assert_eq!(value["status"], "planned");
            assert_eq!(value["checks_executed"], 0);
            chat.workflow_result(value);
            assert!(chat.qa_digest.is_empty() && chat.workflow_digest.is_empty());
        }
        assert!(!app.state.join("qa-plan-state").exists());
        chat.select_mode(0, false);
        assert_eq!(chat.mode, modes::Mode::Feature);
        assert!(chat.qa_id.is_empty() && chat.qa_digest.is_empty());
        let other = base.join("other-project");
        std::fs::create_dir(&other).unwrap();
        forge_domain::git(&other, &["init", "-q"]).unwrap();
        let other_app = App::new(&other, &base.join("other-state"), None, None).unwrap();
        assert!(
            pending(&other_app, &Chat::default(), "/qa-status terminal-qa")
                .unwrap()
                .unwrap()
                .handle
                .await
                .unwrap()
                .is_err()
        );
    }
}
