//! Development modules separate scoped feature execution from pending QA coverage.
pub(crate) mod feature;
pub(crate) mod qa;
use crate::{engine::Context, planning, review};
use anyhow::Result;
use clap::{Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum Command {
    /// Discover, prepare and execute explicitly reviewed QA checks.
    Qa {
        #[command(subcommand)]
        command: qa::Command,
    },
    /// Prepare, execute and resume a feature through the existing bounded workflow.
    Feature {
        #[command(subcommand)]
        command: feature::Command,
    },
    /// List module availability independently of the execution harness.
    List,
    /// Validate feature tasks, dependencies and acceptance without executing them.
    FeaturePlan {
        proposal: PathBuf,
        #[arg(long)]
        root: PathBuf,
    },
    /// Prepare QA coverage for a revision-bound manifest; executes no checks.
    QaPlan {
        manifest: PathBuf,
        #[arg(long, value_enum, default_value = "standard")]
        level: QaLevel,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum QaLevel {
    Light,
    Standard,
    Intensive,
}

fn coverage(level: QaLevel) -> Vec<Value> {
    let checks = [
        (
            "QA-QUALITY",
            QaLevel::Light,
            "quality",
            "Formatting, lint, code smells and existing tests",
        ),
        (
            "QA-SECRETS",
            QaLevel::Light,
            "secrets",
            "Secret scanning with redacted evidence",
        ),
        (
            "QA-DEPENDENCIES",
            QaLevel::Light,
            "supply_chain",
            "Lockfiles, dependency inventory and advisory coverage",
        ),
        (
            "QA-SAST",
            QaLevel::Standard,
            "static_security",
            "Injection, unsafe data flow and regex denial of service",
        ),
        (
            "QA-AUTH",
            QaLevel::Standard,
            "trust_boundaries",
            "Authentication, authorization and tenant isolation",
        ),
        (
            "QA-BUILD",
            QaLevel::Standard,
            "supply_chain",
            "Install scripts, CI permissions, artifact provenance and dependency sources",
        ),
        (
            "QA-ADVERSARIAL",
            QaLevel::Intensive,
            "adversarial",
            "Bounded fuzzing and property tests in a disposable local environment",
        ),
        (
            "QA-THREAT",
            QaLevel::Intensive,
            "threat_model",
            "Abuse cases and cross-component trust boundaries",
        ),
    ];
    checks.into_iter().filter(|(_, minimum, _, _)| *minimum <= level)
        .map(|(id, _, category, objective)| json!({
            "id":id,"category":category,"objective":objective,
            "status":"not_run","applicability":"unknown",
            "tool":null,"evidence":null,
            "required_before_execution":["stack and scope discovery","reviewed command and permissions","pinned external evaluator","time and attempt limits"]
        })).collect()
}

pub async fn dispatch(command: &Command, state_dir: &Path) -> Result<Value> {
    match command {
        Command::Qa { command } => qa::dispatch(command).await,
        Command::Feature { command } => feature::dispatch(command).await,
        Command::List => Ok(json!({"schema_version":1,"modules":[
            {"id":"feature","availability":"execution","priority":1,"harness":"codex","command":"modules feature","planning_command":"modules feature-plan","cycle":["scope","acceptance","small implementation","validation"],"continuity":"existing workflow journal; explicit resume"},
            {"id":"qa","availability":"execution","priority":2,"levels":["light","standard","intensive"],"harness":"codex","command":"modules qa","planning_command":"modules qa-plan"},
            {"id":"refactor","availability":"roadmap","cycle":["behavior baseline","regression tests","structural change","qa"]},
            {"id":"cicd","availability":"roadmap","cycle":["pipeline plan","build","qa","artifact","deployment approval","deploy","verify","rollback decision"]}
        ],"native_call":false,"state":"not_modified","acceptance":"unchanged"})),
        Command::FeaturePlan { proposal, root } => {
            let candidate = planning::validate_file(proposal, root)?;
            Ok(
                json!({"schema_version":1,"module":"feature","status":"planned",
                "candidate":candidate,"native_call":false,"state":"not_modified",
                "acceptance":"unchanged","execution":"not_started",
                "next_steps":["Review the task DAG and observable acceptance criteria","Bind reviewed commands to external pinned evaluators through the existing workflow","Run bounded implementation and validation using the reviewed contract"],
                "limitations":["Proposed commands are untrusted data and are not executed","The full QA module and automatic context checkpoint loop are subsequent increments"]}),
            )
        }
        Command::QaPlan { manifest, level } => {
            let context = Context::load(manifest, state_dir)?;
            let existing = review::plan(&context)?;
            Ok(json!({"schema_version":1,"module":"qa","level":level,
                "status":"planned","execution_available":false,
                "review_plan":existing["review_plan"],"coverage":coverage(*level),
                "loop":{"status":"design_only","stages":["discover","plan","validate","review","prioritize","authorized correction","revalidate"],
                    "stop_on":["budget exhausted","attempt limit","unchanged failure","missing evidence","uncertain external result"],
                    "limits_source":"existing manifest review and release budgets"},
                "checkpoint":{"status":"design_only","boundary":"completed work unit or context pressure before next native call",
                    "required":["source revision","manifest and evaluator hashes","objective and acceptance criteria","decisions","evidence references","pending tasks","incomplete work","remaining budget"],
                    "resume":"revalidate source and evidence; changed inputs require reconciliation"},
                "native_call":false,"checks_executed":0,"state":"not_modified","acceptance":"unchanged",
                "limitations":["No scanners or repository scripts executed","Coverage is pending, not a security attestation","Levels never expand permissions or authorize deployment","Native review alone does not execute this coverage plan"]}))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deeper_levels_retain_baseline_without_claiming_evidence() {
        let light = coverage(QaLevel::Light);
        let standard = coverage(QaLevel::Standard);
        let intensive = coverage(QaLevel::Intensive);
        assert!(light.len() < standard.len() && standard.len() < intensive.len());
        for check in &light {
            assert!(standard.contains(check));
        }
        for check in &standard {
            assert!(intensive.contains(check));
        }
        for check in intensive {
            assert_eq!(check["status"], "not_run");
            assert_eq!(check["applicability"], "unknown");
            assert!(check["evidence"].is_null());
        }
    }
}
