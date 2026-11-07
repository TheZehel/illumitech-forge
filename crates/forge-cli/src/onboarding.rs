//! Bounded prerequisite checks, without inference or installation.
use crate::{
    diagnostics::{action, blocked},
    guided, installation,
};
use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;

pub async fn doctor(root: &Path) -> Result<Value> {
    let signals = crate::publication::Signals::new();
    let mut checks = vec![];
    let project = guided::root(root);
    checks.push(json!({"id":"project","passed":project.is_ok(),"status":if project.is_ok(){"available"}else{"missing"},
        "reason":"An existing Git worktree is required for native workflows",
        "action":action("Choose an existing Git project, or create a disposable demo", None, true)}));
    let cwd = if root.is_dir() {
        root.to_path_buf()
    } else {
        std::env::current_dir()?
    };
    let mut auth_kind = "unavailable";
    for (id, argv, guidance) in [
        (
            "git",
            vec!["git".into(), "--version".into()],
            "Install Git and reopen your terminal",
        ),
        (
            "client",
            vec!["codex".into(), "--version".into()],
            "Install Codex CLI separately and reopen your terminal",
        ),
        (
            "authentication",
            vec!["codex".into(), "login".into(), "status".into()],
            "Authenticate Codex CLI using its native login flow",
        ),
    ] {
        let result = forge_adapters::process::run(&argv, &cwd, 10, signals.receiver.clone()).await;
        let (passed, status, reason) = match result {
            Ok(r) if r.passed() => {
                if id == "authentication" {
                    auth_kind = if format!("{}{}", r.stdout, r.stderr)
                        .to_lowercase()
                        .contains("chatgpt")
                    {
                        "subscription"
                    } else {
                        "native_configured"
                    };
                }
                (true, "available", "Bounded native check passed")
            }
            Ok(_) => (false, "unavailable", guidance),
            Err(ref e)
                if e.chain().any(|e| {
                    e.downcast_ref::<std::io::Error>()
                        .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound)
                }) =>
            {
                (false, "missing", guidance)
            }
            Err(_) => (false, "unverified", guidance),
        };
        checks.push(json!({"id":id,"passed":passed,"status":status,"reason":reason,"action":action(if passed { "No action required for this prerequisite" } else { guidance },None,!passed)}));
    }
    let install = installation::dispatch(&installation::Command::Doctor).await?;
    let paths = install["path_diagnostics"]["commands"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let path_attention = paths
        .iter()
        .any(|p| matches!(p["status"].as_str(), Some("attention" | "incomplete")));
    checks.push(json!({"id":"installation","passed":!path_attention,"status":if path_attention{"attention"}else{"available"},
        "reason":"Running version is known; other PATH candidate versions remain unknown",
        "action":action("Inspect forge installation doctor before updating or changing PATH",Some(vec!["forge".into(),"installation".into(),"doctor".into()]),true)}));
    let healthy = checks.iter().all(|c| c["passed"] == true);
    let mut value = json!({"schema_version":1,"status":if healthy{"passed"}else{"blocked"},"checks":checks,
        "authentication_kind":auth_kind,"installation":install,"model":"native configuration unchanged; not inferred by this diagnostic",
        "native_inference_calls":0,"mutation":false,"memory":"optional",
        "platform":{"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"status":"unverified",
          "reason":"Host detection is not native-account or editor acceptance; consult the release evidence matrix"},
        "onboarding":{"schema_version":1,"paths":[
          {"id":"demo","requirements":["git"],"next_action":"forge demo create NEW_DIRECTORY; inspect the returned command arrays before running them"},
          {"id":"feature","requirements":["git","project","client","authentication"],"next_action":"Review a Feature request, prepare it, then authorize its exact scope"}],
          "installation":"Choose a verified platform artifact or build from source. Install only when requested; never change native account/model settings."},
        "next_action":if healthy{"Review your workflow before execution"}else{"Resolve each missing prerequisite, then rerun this read-only diagnostic"}});
    if !healthy {
        value["diagnostic"] = serde_json::to_value(blocked(
            "prerequisite_unavailable",
            "One or more workflow prerequisites require attention",
            "workflow doctor",
            "Inspect the per-check actions; the no-agent demo needs only Git",
        ))?;
    }
    Ok(value)
}
