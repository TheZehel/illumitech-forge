//! Native entrypoint defaults. No shell shim, inference or implicit project initialization.
use crate::{Cli, Command, guided, terminal_chat};
use anyhow::{Context, Result, ensure};
use std::{
    ffi::OsString,
    io::IsTerminal,
    path::{Path, PathBuf},
};

pub fn arguments(mut args: Vec<OsString>) -> (String, Vec<OsString>) {
    let short = args
        .first()
        .and_then(|a| Path::new(a).file_stem())
        .is_some_and(|n| n == "forge");
    if !short {
        return ("illumitech-forge".into(), args);
    }
    let mut value = false;
    let mut explicit = false;
    for arg in args.iter().skip(1) {
        if value {
            value = false;
            continue;
        }
        let arg = arg.to_string_lossy();
        if matches!(arg.as_ref(), "--state-dir" | "--root" | "--app-server") {
            value = true;
            continue;
        }
        if matches!(arg.as_ref(), "--help" | "-h" | "--version" | "-V") || !arg.starts_with('-') {
            explicit = true;
            break;
        }
    }
    if !explicit {
        args.insert(1, "chat".into());
    }
    ("forge".into(), args)
}
fn project_root(path: &Path) -> Result<Option<PathBuf>> {
    let path = guided::safe_path(path)?.canonicalize()?;
    ensure!(path.is_dir(), "project path must be a directory");
    Ok(path
        .ancestors()
        .find(|p| p.join(".git").exists())
        .map(Path::to_path_buf))
}
fn projects(parent: &Path) -> Result<Vec<PathBuf>> {
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(parent)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() && entry.path().join(".git").exists() {
            candidates.push(guided::root(&entry.path())?);
        }
    }
    candidates.sort();
    Ok(candidates)
}
fn resolve_project(path: &Path, allow_picker: bool) -> Result<PathBuf> {
    if let Some(root) = project_root(path)? {
        return Ok(root);
    }
    ensure!(
        allow_picker,
        "a pasta selecionada não pertence a um projeto Git"
    );
    ensure!(
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal(),
        "nenhum projeto Git na pasta atual; use forge --root CAMINHO em um terminal interativo"
    );
    let candidates = projects(&path.canonicalize()?)?;
    ensure!(
        !candidates.is_empty(),
        "nenhum projeto Git nesta pasta; entre em um projeto ou use forge --root CAMINHO"
    );
    terminal_chat::choose_project(&candidates)
}
fn default_state_base() -> Result<PathBuf> {
    #[cfg(windows)]
    let path = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .context("LOCALAPPDATA unavailable; provide --state-dir")?
        .join("IllumitechForge/State");
    #[cfg(not(windows))]
    let path = if let Some(path) = std::env::var_os("XDG_STATE_HOME").filter(|p| !p.is_empty()) {
        PathBuf::from(path).join("illumitech-forge")
    } else {
        PathBuf::from(std::env::var_os("HOME").context("HOME unavailable; provide --state-dir")?)
            .join(".local/state/illumitech-forge")
    };
    ensure!(
        path.is_absolute(),
        "state base must be absolute; provide --state-dir"
    );
    Ok(path)
}
fn scoped_state(base: &Path, root: &Path) -> PathBuf {
    let hash = forge_domain::digest(root.as_os_str().as_encoded_bytes());
    base.join("workbench").join(hash)
}
pub fn prepare(cli: &mut Cli, explicit_state: bool, explicit_root: bool) -> Result<()> {
    let root = match &mut cli.command {
        Command::Chat { root, .. } => {
            *root = resolve_project(root, !explicit_root)?;
            Some(root.clone())
        }
        Command::Workbench { root, .. } => Some(guided::root(root)?),
        _ => None,
    };
    if let Some(root) = root
        && !explicit_state
    {
        cli.state_dir = scoped_state(&default_state_base()?, &root);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(value: &[&str]) -> Vec<OsString> {
        value.iter().map(OsString::from).collect()
    }
    #[test]
    fn shorthand_keeps_help_commands_and_option_values_distinct() {
        for input in [
            vec!["forge"],
            vec!["forge", "--root", "doctor"],
            vec!["forge", "--state-dir", "chat"],
        ] {
            let (_, actual) = arguments(args(&input));
            assert_eq!(actual[1], "chat");
        }
        for input in [
            vec!["forge", "--help"],
            vec!["forge", "--version"],
            vec!["forge", "release", "inspect", "r.yaml"],
            vec!["illumitech-forge"],
            vec!["illumitech-forge", "doctor"],
        ] {
            assert_eq!(arguments(args(&input)).1, args(&input));
        }
        assert_eq!(
            arguments(args(&["/bin/forge.exe"])).1,
            args(&["/bin/forge.exe", "chat"])
        );
    }
    #[test]
    fn nested_project_discovers_root_and_scopes_are_isolated_without_writes() {
        let t = tempfile::tempdir().unwrap();
        let base = t.path().canonicalize().unwrap();
        let a = base.join("project a");
        let b = base.join("project b");
        for p in [&a, &b] {
            std::fs::create_dir_all(p.join(".git")).unwrap();
        }
        std::fs::create_dir_all(a.join("src/nested")).unwrap();
        assert_eq!(
            project_root(&a.join("src/nested")).unwrap(),
            Some(a.clone())
        );
        assert_ne!(scoped_state(&base, &a), scoped_state(&base, &b));
        assert_eq!(projects(&base).unwrap(), vec![a, b]);
        assert!(!base.join("workbench").exists());
    }
    #[test]
    fn explicit_state_is_preserved_and_explicit_nonproject_is_not_selected_implicitly() {
        use clap::Parser;
        let t = tempfile::tempdir().unwrap();
        let base = t.path().canonicalize().unwrap();
        let root = base.join("project");
        std::fs::create_dir_all(root.join(".git")).unwrap();
        let mut cli = Cli::parse_from([
            "illumitech-forge",
            "--state-dir",
            "chosen-external-state",
            "chat",
            "--root",
            root.to_str().unwrap(),
        ]);
        prepare(&mut cli, true, true).unwrap();
        assert_eq!(cli.state_dir, PathBuf::from("chosen-external-state"));
        assert!(resolve_project(&base, false).is_err());
    }
}
