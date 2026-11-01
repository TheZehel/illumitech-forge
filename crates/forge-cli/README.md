# Illumitech Forge CLI 1.1.1

Evidence-backed release execution, local web workbench and native interactive terminal.

```sh
cargo install illumitech-forge-cli --version 1.1.1 --locked
forge
```

`forge` opens the terminal conversation in your Git project; outside a project it offers an
explicit project picker. `forge --root PATH` selects another project. State defaults to an
external per-project directory. `illumitech-forge` remains available for existing scripts.

The terminal includes an ASCII identity, Markdown, `/` command menu, F1 help, `/status`,
Ctrl+R session history, readline editing and native permission decisions. Managed native
Codex authentication is preserved; Forge does not silently switch accounts, models or APIs.
ai-memory is optional. Chat text is not gate evidence, and publication is always explicit.

See the repository README for manifests, workbench, installation, support limits and credits.
Windows and Claude remain experimental; this release's binary package targets macOS ARM64.
Code is MIT; third-party notices and dependency inventory accompany the packages.

## First execution without an agent

```sh
forge demo create /path/to/a-new-directory
```

Creation prepares an example, without running it. The response includes inspect, dry-run,
run and report commands using the same external state directory. Only Git is needed; the
native evaluator is pinned by SHA-256 and verifies an artifact and preserved file.

`init` creates no memory routing by default. Explicit `init --memory-workspace ID` is opt-in
and preserves existing configuration. `installation doctor` inspects both executable names
without running unknown candidates. Status/report return 2 for stale evidence even if the
historical run snapshot remains passed. See the release verification for support limits.
