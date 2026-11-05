//! Feature-specific admission; the existing workflow owns execution and durable state.
use crate::{guided, planning, workflow};
use anyhow::{Context as _, Result, ensure};
use clap::Subcommand;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum Command {
    /// Preview exact feature scope; --apply saves an external candidate, not execution.
    Prepare {
        input: PathBuf,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        ops_root: PathBuf,
        #[arg(long)]
        apply: bool,
    },
    /// Execute the reviewed feature scope and its per-task external checks.
    Start {
        id: String,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        ops_root: PathBuf,
        #[arg(long)]
        approve_scope: String,
    },
    /// Inspect current evidence and the persisted workflow journal without replay.
    Status {
        id: String,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        ops_root: PathBuf,
    },
    /// Resume only through the existing revision/evaluator reconciliation checks.
    Resume {
        id: String,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        ops_root: PathBuf,
        #[arg(long)]
        reconcile: Option<String>,
    },
    /// Project a context handoff from durable state; never approves or resumes work.
    Checkpoint {
        id: String,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        ops_root: PathBuf,
    },
}

pub(crate) fn validate(request: &workflow::Request, root: &Path) -> Result<()> {
    ensure!(
        request.recipe == "feature",
        "feature module requires recipe=feature"
    );
    let proposal = request
        .proposal
        .as_ref()
        .context("feature module requires a structured proposal")?;
    planning::validate(proposal, root)?;
    ensure!(
        proposal.id == request.id && proposal.bug.is_none(),
        "feature proposal identity/type mismatch"
    );
    for task in &proposal.tasks {
        for criterion in &task.acceptance {
            let check = request
                .checks
                .iter()
                .find(|check| check.id == criterion.id)
                .with_context(|| {
                    format!("criterion {} has no explicit external check", criterion.id)
                })?;
            ensure!(
                check.argv == criterion.command
                    && (check.task_ids.is_empty() || check.task_ids.contains(&task.id))
                    && check.before_exit.is_none(),
                "criterion {} must match its external check command and task scope",
                criterion.id
            );
        }
    }
    Ok(())
}

fn checked(root: &Path, ops: &Path, id: &str) -> Result<workflow::Candidate> {
    let (_, candidate) = workflow::load(root, ops, id)?;
    validate(&candidate.request, &candidate.root)?;
    Ok(candidate)
}

pub async fn dispatch(command: &Command) -> Result<Value> {
    let mut value = match command {
        Command::Prepare {
            input,
            root,
            ops_root,
            apply,
        } => {
            let request = workflow::read_json(input)?;
            validate(&request, &guided::root(root)?)?;
            workflow::prepare::prepare(request, root, ops_root, *apply)?
        }
        Command::Start {
            id,
            root,
            ops_root,
            approve_scope,
        } => {
            checked(root, ops_root, id)?;
            workflow::start(root, ops_root, id, approve_scope).await?
        }
        Command::Resume {
            id,
            root,
            ops_root,
            reconcile,
        } => {
            checked(root, ops_root, id)?;
            workflow::dispatch(&workflow::Command::Resume {
                id: id.clone(),
                root: root.clone(),
                ops_root: ops_root.clone(),
                reconcile: reconcile.clone(),
            })
            .await?
        }
        Command::Status { id, root, ops_root } | Command::Checkpoint { id, root, ops_root } => {
            let candidate = checked(root, ops_root, id)?;
            let mut status = workflow::dispatch(&workflow::Command::Status {
                id: id.clone(),
                root: root.clone(),
                ops_root: ops_root.clone(),
            })
            .await?;
            if matches!(command, Command::Checkpoint { .. }) {
                status["checkpoint"] = json!({
                    "schema_version":1,"kind":"read_only_projection","authority":"none",
                    "objective":candidate.request.request,"proposal":candidate.request.proposal,
                    "scope_digest":workflow::consent_digest(&candidate)?,
                    "prepared_source":candidate.source,
                    "current_source":forge_domain::revision(&candidate.root)?,
                    "manifest_contract_sha256":forge_domain::digest(&serde_json::to_vec(&candidate.manifest)?),
                    "evaluator_sha256":candidate.evaluator_sha256,
                    "context_policy":candidate.manifest.context,
                    "limits":{"release":candidate.manifest.budget,"tasks":candidate.manifest.tasks.iter().map(|t|json!({"id":t.id,"timeout_seconds":t.timeout_seconds,"max_attempts":t.max_attempts,"budget":t.budget})).collect::<Vec<_>>()},
                    "observed_state":"See tasks, report and journal_metrics in this response; missing usage is unknown",
                    "durable_source":"external workflow candidate, checks, manifest and state journal",
                    "resume_requires":"Reload durable state and revalidate source/evaluator through modules feature resume",
                    "automatic_compaction":false,"mutation":false
                });
            }
            status
        }
    };
    value["module"] = json!("feature");
    Ok(value)
}
