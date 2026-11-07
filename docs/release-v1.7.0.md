# Illumitech Forge 1.7.0

First-run diagnostics, parser-derived command discovery, typed operational explanations
and durable continuity help users and assistants operate the same Forge engine. The
portable skill covers setup, Feature, QA, recovery and explicitly authorized CLI operations.

- `forge --json workflow doctor --root .` checks prerequisites and suggests demo or Feature
  routes without inference, installation or login.
- `forge --json contracts --command "modules feature prepare"` exposes the actual public
  parser arguments, defaults and conservative operation effects.
- CLI, terminal and Workbench explain supported blocker categories and next steps. Unknown
  failures remain explicitly unclassified; suggestions do not authorize recovery.
- Workflow, Feature and QA status share objective, progress, coverage gaps and continuity.
  Stale, active or uncertain work requires review before replay.
- Standalone skill and Agent Plugin ZIPs include file checksums. The binary archive also
  carries the public skill source. Codex remains the native executor.

Existing state and manifest contracts remain schema 1 with additive output fields. Existing
error/error_code and next_action shapes are retained. CLI and skill evolve together; there
is no new telemetry. See [setup and contracts](agent-experience.md).

Validation includes 307 Rust unit/integration tests, workspace doc tests, formatting,
Clippy with warnings denied, documentation, UI build/typecheck and four Chrome scenarios.
The native Codex Feature pilot passed against a debug artifact with an external check;
the separately built release artifact passed extracted-package and browser checks.

Platform evidence remains macOS ARM64. Codex CLI discovered the project skill. A limited
VS Code pilot observed skill discovery in the Codex extension and version 1.7.0 in its
integrated terminal. A complete model-driven IDE workflow and directory installation were
not tested. The Codex IDE extension supports standalone skills, not plugins. Cursor was
absent and remains unvalidated. Linux native acceptance remains deferred; Windows and
Claude remain experimental. No new platform acceptance is claimed.

GitHub release assets include macOS ARM64 binaries, four source crates, a source archive,
a local site preview, the skill/plugin ZIPs, verification metadata and SHA256SUMS. Marketplace
submissions, global CLI upgrades and website deployment are separate operations.
