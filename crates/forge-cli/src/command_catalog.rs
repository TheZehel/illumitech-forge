//! Discover syntax from the parser, with conservative, explicitly reviewed effects.
use clap::Command;
use serde_json::{Value, json};

fn effect(path: &str) -> &'static str {
    match path {
        "contracts"
        | "installation doctor"
        | "doctor"
        | "workflow doctor"
        | "workflow status"
        | "workflow diagnose"
        | "workflow list"
        | "workflow attempts"
        | "workflow recipes"
        | "workflow findings"
        | "modules list"
        | "modules feature-plan"
        | "modules qa-plan"
        | "modules feature status"
        | "modules feature checkpoint"
        | "modules qa status"
        | "modules qa discover"
        | "release inspect"
        | "release version"
        | "status"
        | "report"
        | "context plan"
        | "review plan"
        | "budget inspect"
        | "tools list"
        | "tools doctor"
        | "tools plan"
        | "spec check"
        | "memory doctor"
        | "memory query"
        | "eval compare"
        | "daemon-control status" => "read_only",
        "workflow prepare"
        | "modules feature prepare"
        | "modules qa prepare"
        | "workflow export"
        | "feature new"
        | "plan import"
        | "release import-loop"
        | "release import-spec"
        | "installation update"
        | "service" => "preview_or_write",
        "release publish" | "release draft-pr" => "publication_requires_authorization",
        "workflow start"
        | "workflow resume"
        | "modules feature start"
        | "modules feature resume"
        | "modules qa start"
        | "release run"
        | "eval run"
        | "resume"
        | "daemon" => "execution_requires_review",
        "chat" | "workbench" | "dashboard" => "local_interactive_service",
        // All remaining operations may write or invoke external tools. Never imply read-only.
        _ => "review_required",
    }
}
fn entry(command: &Command, path: &str) -> Value {
    let args = command.get_arguments().filter(|a| !a.is_hide_set()).map(|a| json!({
        "id":a.get_id().as_str(),"long":a.get_long(),"short":a.get_short().map(|c|c.to_string()),
        "required":a.is_required_set(),"positional":a.is_positional(),
        "takes_values":a.get_action().takes_values(),"help":a.get_help().map(|s|s.to_string()),
        "value_names":a.get_value_names().map(|v|v.iter().map(|s|s.as_str()).collect::<Vec<_>>()),
        "default_values":a.get_default_values().iter().map(|s|s.to_string_lossy()).collect::<Vec<_>>()
    })).collect::<Vec<_>>();
    let mut copy = command.clone();
    json!({"path":path,"about":command.get_about().map(|s|s.to_string()),
        "usage":copy.render_usage().to_string(),"arguments":args,
        "subcommands":command.get_subcommands().filter(|c|!c.is_hide_set()).map(|c|c.get_name()).collect::<Vec<_>>(),
        "effect":effect(path),"execution_authorized":false})
}
fn walk(command: &Command, prefix: &str, out: &mut Vec<Value>) {
    for child in command
        .get_subcommands()
        .filter(|c| !c.is_hide_set() && c.get_name() != "help")
    {
        let path = format!("{prefix}{}", child.get_name());
        out.push(entry(child, &path));
        walk(child, &format!("{path} "), out);
    }
}
pub fn catalog(mut parser: Command, selection: Option<&str>) -> anyhow::Result<Value> {
    parser.build();
    let mut commands = vec![];
    walk(&parser, "", &mut commands);
    if let Some(selection) = selection {
        let selection = selection.split_whitespace().collect::<Vec<_>>().join(" ");
        commands.retain(|c| c["path"] == selection);
        anyhow::ensure!(
            !commands.is_empty(),
            crate::diagnostics::blocked(
                "unknown_command",
                "Unknown public command path",
                "contracts",
                "Consult forge --json contracts for public command paths"
            )
        );
    }
    Ok(
        json!({"schema_version":1,"source":"clap_parser","commands":commands,
        "effects_note":"Effects are conservative; options such as --apply or --dry-run refine behavior. Discovery never grants authorization."}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    #[test]
    fn catalog_uses_real_arguments_and_hides_internal_commands() {
        let catalog = catalog(crate::Cli::command(), None).unwrap();
        assert!(
            !catalog["commands"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["path"].as_str().unwrap().contains("service-host"))
        );
        let c = super::catalog(crate::Cli::command(), Some("workflow prepare")).unwrap();
        assert!(
            c["commands"][0]["arguments"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["long"] == "ops-root" && a["required"] == true)
        );
    }
    #[test]
    fn documented_examples_parse_without_executing() {
        for args in [
            vec!["contracts", "--command", "modules feature prepare"],
            vec!["workflow", "doctor", "--root", "/project"],
            vec![
                "modules",
                "feature",
                "prepare",
                "/ops/request.json",
                "--root",
                "/project",
                "--ops-root",
                "/ops",
            ],
            vec![
                "modules",
                "qa",
                "status",
                "qa-id",
                "--root",
                "/project",
                "--ops-root",
                "/ops",
            ],
            vec![
                "modules",
                "feature",
                "resume",
                "feature-id",
                "--root",
                "/project",
                "--ops-root",
                "/ops",
            ],
            vec!["demo", "create", "/tmp/new-demo"],
            vec!["release", "inspect", "/ops/release.yaml"],
        ] {
            crate::Cli::command()
                .try_get_matches_from(std::iter::once("forge").chain(args))
                .unwrap();
        }
    }
}
