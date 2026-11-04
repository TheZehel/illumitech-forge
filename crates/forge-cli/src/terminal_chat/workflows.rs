//! Explicit terminal workflow commands; chat messages never execute a release implicitly.
use super::*;
use crate::workflow;
use anyhow::Context as _;

pub(super) fn pending(app: &Arc<App>, chat: &Chat, text: &str) -> Result<Option<Pending>> {
    let command = text.split_whitespace().next().unwrap_or("").to_owned();
    if !["/bug", "/plan", "/run", "/tasks", "/report"].contains(&command.as_str()) {
        return Ok(None);
    }
    let args = text
        .strip_prefix(command.as_str())
        .unwrap_or("")
        .trim()
        .to_owned();
    let current = chat.workflow_id.clone();
    let digest = chat.workflow_digest.clone();
    let app = Arc::clone(app);
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce).map_err(|e| anyhow::anyhow!("operation ID: {e}"))?;
    let operation_id = format!("terminal-workflow-{}", forge_domain::digest(&nonce));
    let route = format!("workflow{command}");
    let handle = tokio::spawn(async move {
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
    }))
}
