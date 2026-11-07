# Setup and first run

Check the selected executable's path and `--version`. If absent, inspect OS/architecture and
Git availability using the host's read-only tools. Present the verified release artifact or
Cargo build option. Official source: https://github.com/TheZehel/illumitech-forge . Download
artifacts and SHA256SUMS from the same release. Do not infer support from the editor/platform.
Do not remove competing binaries or change PATH automatically. A terminal may need reopening.

Install only when requested, using the selected compatible published version. A candidate
skill may require a CLI version not yet published: explain that and offer the reviewed source
build or wait for release. Never present an unpublished version as available on crates.io.
The source build is `cargo build --locked -p illumitech-forge-cli --bins`; it does not install.
Cargo needs Rust 1.94.0. Native Codex is installed and authenticated separately; no model/API fallback.

For an existing project:

```
forge --json installation doctor
forge --json workflow doctor --root /absolute/project
```

Inspect each check, installation.path_diagnostics, platform limitations and onboarding paths.
Do not run candidates found on PATH merely to discover their versions. Doctor does bounded
native version/login-status checks; it does not run inference, install tools or authenticate.

The no-agent demo needs only Git: `forge --json demo create /absolute/new-demo`.
The destination must be new. Review returned argv arrays `commands.inspect`, `dry_run`, `run`
and `report`. Execute using the actual selected binary, preserving every argument and path.
Inspection and dry-run must pass before the authorized execution. Verify both output and the
preserved unrelated file. This synthetic demo does not prove native account or editor support.

For a real first Feature, use the development reference and the bundled feature-request.json
in a disposable Git project. Its Python check requires Python 3; it is an example dependency,
not a requirement for opening Forge. Keep operations/request files outside the project.
