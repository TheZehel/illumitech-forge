# Agent experience — 1.7.0

Version 1.7.0 adds a common interface for the CLI, portable skill and existing Workbench.
The release notes distinguish native execution, skill discovery and editor workflow evidence.
Packages and successful tests alone do not establish acceptance in every host or platform.

## First run / Primeira execução

Inside an existing Git project, run `forge --json workflow doctor --root .`.
It reports project/Git, native client and authentication prerequisites, the Git revision
needed by Feature preparation, the running version, PATH attention and platform uncertainty.
An unborn repository needs an initial commit reviewed by its operator; doctor never creates
that commit. Published version 1.7.0 predates the separate git_revision check and can first
expose this absence during preparation. Version/login-status and Git revision probes have
bounded timeouts; there is no inference, login, installation or state creation.
`forge installation doctor` never runs other binaries found on PATH.

There are two routes. **Demo:** only Git, new directory, then inspect the command arrays
returned by `forge --json demo create NEW_DIRECTORY`; inspect/dry-run precede execution.
**Feature:** separately install/authenticate Codex, review the external request/checks and
prepare/authorize a bounded Feature. The small bundled Feature example needs Python 3;
opening the terminal or Workbench does not. Quickstart installation commands select the
version declared in release/current.json; check the public release before installation.

O diagnóstico mostra o requisito ausente e a ação correspondente. Use a demonstração sem
agente para conhecer o ciclo e uma Feature pequena para validar a conta nativa. O botão
**Verificar ambiente** no Workbench consulta o mesmo diagnóstico. Modelo, permissões e
conta permanecem gerenciados pelo Codex. O CLI continua disponível sem nenhuma skill.

## Public contracts

`forge --json contracts` adds command_catalog schema 1, derived from the real Clap parser.
`forge --json contracts --command "modules feature prepare"` filters an exact public command
path. Hidden/internal commands are omitted; parameters, required options, defaults and help
come from the parser. Effects are conservative classifications: read_only, preview_or_write,
execution_requires_review, publication_requires_authorization, local_interactive_service or
review_required. Option-dependent behavior still requires help/contract review.

Operational errors retain error/error_code and add diagnostic schema 1: code, cause,
operation, actions (reason, nullable argv, requires_review) and authority=none. Diagnostics
originate in typed checks, never error-string matching. Unclassified failures return
`diagnosis_unavailable`; partial diagnosis is not a comprehensive error taxonomy. Known
categories include unknown command, invalid input, missing project/prerequisites, changed
scope/source/evaluator, active process and required reconciliation. Existing parser failures
remain on stderr with exit 2, outside the JSON envelope. Existing next_action shapes remain.

Workflow/Feature/QA status adds continuity schema 1: kind/id/objective, status/stop_reason,
progress with observed items, coverage_gaps, nullable evidence_current, references,
diagnostic and next_actions. Null means unknown. It reads current candidate/journal evidence;
no new database or transferable authorization is created. Inspect actions contain argv arrays;
review-dependent steps do not fabricate a runnable authorization command.

Active, uncertain or stale work never implies replay. Feature's existing checkpoint retains
its contract. QA scopes execute once and have no resume command. Incomplete QA remains
`degraded`. Terminal and Workbench show the shared cause and next step; original reports are
still accessible. Suggestions are not buttons that automatically execute recovery.

## Portable skill and packages

The source of truth is `skills/illumitech-forge`. Requires CLI >=1.7.0,<2.0.0 and the new
schema-1 capabilities. Install exactly one copy in the selected host; do not also install
skills/core as a public skill. Core remains internal repository governance.

Build two reproducible packages in a fresh output directory:

```
python3 scripts/package_agent_skill.py --output /absolute/new-artifacts
```

The standalone archive contains an illumitech-forge folder. For local development, copy that
folder into `.agents/skills` in a Codex project (also available to its VS Code extension),
or `.cursor/skills` in a Cursor project. Do not overwrite an existing installation. Reopen
or refresh the host and check discovery. This local setup is distinct from marketplace
installation. The Codex IDE extension supports standalone skills but does not support
plugins. For source-repository skill installers, select only `illumitech-forge`.

The plugin archive has root plugin.json plus skills/illumitech-forge and MIT. OpenAI metadata
marks local execution required. It has no MCP server, hooks, binaries, credentials or automatic
installer. The portable package is also intended for Cursor's Agent Plugins support; actual
host installation/discovery must be piloted before claiming compatibility. A Cursor host
still invokes Forge; Forge's native execution remains Codex.

Both archives include file checksums and have external SHA-256 sidecars plus SHA256SUMS.
They must match workspace/plugin versions. Rebuilding identical inputs produces identical
bytes. Existing output directories are refused. The ordinary binary packaging script includes
these docs and skill source; the skill ZIPs are additional release assets.

Codex CLI discovery and a disposable native Feature workflow passed. The first isolated
VS Code pilot observed project-skill discovery and terminal access. A post-publication
pilot then completed a bounded Feature through the Codex extension using the published
macOS binary: initial and final external gates passed, exact greeting bytes and all eight
preserved files were independently checked, and status/checkpoint reported current evidence.
The editor's normal per-command approvals allowed the reviewed external ops directory.

The same bounded flow passed in a temporary Cursor 3.29.9 profile through the Codex extension:
project-skill discovery, native Feature execution, both external gates, exact bytes,
preservation and current status/checkpoint. The user completed login and the extension tour.
Native Cursor chat and the Codex extension are distinct interfaces; the native Cursor/Grok
agent was not used. Plugin-directory installation was not tested. This evidence covers those
small local Feature flows, rather than every module or platform. See
[host evidence](../specs/066-agent-experience/validation.md) and issue #12.

## Release and feedback

Update release/changes.json against the last published version; check the computed target,
package versions and current skill requirements. Revalidate examples and packages whenever
commands or capability meanings change. Advance release/current.json only in the reviewed
publication revision; local preparation is not proof that the provider has published it.

For every advertised host, record a clean-install discovery and main-flow pilot. Preparing
packages does not submit them. OpenAI desktop-only plugins use the local-executor submission
flow; Cursor plugins require marketplace review. skills.sh can discover/install public skills.
See https://developers.openai.com/plugins/deploy/desktop-only ,
https://cursor.com/docs/plugins and https://skills.sh/docs . Publisher identity and final
revision approval are release tasks, not inferred from a candidate version.

Collect issues/voluntary feedback with CLI/skill versions, host, sanitized reproduction,
expected result and observed difficulty. Reproduce, add a behavioral regression and update
CLI and skill together. There is no new telemetry. Never attach credentials, local session
URLs or raw private journals to a public report.
