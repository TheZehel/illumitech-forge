use super::*;
use forge_adapters::process;

pub(super) async fn run(
    path: &Path,
    id: &str,
    root: &Path,
    expected: Option<i32>,
) -> Result<Value> {
    let root = guided::root(root)?;
    let path = guided::external(path, &root)?;
    let contract: CheckContract = read_json(&path)?;
    ensure!(
        contract.schema_version == 1 && contract.root == root,
        "external check project mismatch"
    );
    for (path, expected) in &contract.scanner_inputs {
        ensure!(
            digest(&std::fs::read(guided::external(path, &root)?)?) == *expected,
            "scanner executable changed; prepare a fresh scope"
        );
    }
    let check = contract
        .checks
        .iter()
        .find(|c| c.id == id)
        .context("unknown external check")?;
    // External source is immutable operator data. Native agents cannot replace it with prose.
    let files = path.parent().context("check parent")?.join("regressions");
    for (name, text) in &contract.files {
        ensure!(
            Path::new(name).file_name().and_then(|s| s.to_str()) == Some(name)
                && !name.starts_with('.')
                && !name.contains(['\\', '/']),
            "unsafe regression filename"
        );
        guided::create(&files.join(name), text)?;
    }
    let argv = check
        .argv
        .iter()
        .map(|a| -> Result<String> {
            Ok(if a == "{repo}" {
                root.to_string_lossy().into_owned()
            } else if let Some(name) = a.strip_prefix("{checks}/") {
                ensure!(
                    contract.files.contains_key(name),
                    "unknown frozen regression"
                );
                files.join(name).to_string_lossy().into_owned()
            } else {
                a.clone()
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let signal = crate::publication::Signals::new();
    let result = process::run(&argv, &root, check.timeout_seconds, signal.receiver.clone()).await?;
    ensure!(
        result.code == Some(expected.unwrap_or(0))
            && result.reason == "exited"
            && !result.truncated,
        "verification {id} failed or incomplete: {} (exit {:?}); {}",
        result.reason,
        result.code,
        result.stderr
    );
    if check.rescan {
        let sarif: Value =
            serde_json::from_str(&result.stdout).context("rescan must emit complete SARIF JSON")?;
        let findings = crate::findings::parse(&sarif, &root)?;
        ensure!(
            findings["status"] == "imported"
                && findings["findings"].as_array().is_some_and(Vec::is_empty),
            "rescan failed or still reports findings; remission is not accepted"
        );
    }
    Ok(
        json!({"schema_version":1,"check":id,"status":"passed","expected_exit":expected.unwrap_or(0),"actual_exit":result.code,"stdout":result.stdout,"stderr":result.stderr,"evidence":"external frozen verifier; not agent prose"}),
    )
}
