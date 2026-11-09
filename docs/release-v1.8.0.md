# Illumitech Forge 1.8.0

This compatible feature release makes partial GitHub publication recoverable and adds
verified, project-scoped installation of the optional public Forge skill. The CLI remains
independently usable; native execution keeps the existing Codex account and selected model.

- Release publication records a separate intent journal tied to source, evidence, tag,
  notes, visibility and asset bytes. It reconciles uncertain create/upload/publish results,
  reuses matching assets and verifies downloads before exposing a complete release.
  Conflicting content, changed inputs, stale approvals and unexpected assets block writes.
- Recovery resolves the actual remote tag commit, including annotated tags; a symbolic
  GitHub `target_commitish` is not source identity. Matching published releases reconcile
  without remote writes. The additive `contracts.release_publication=1` capability gates
  recovery guidance; published 1.7.0/1.7.1 require manual inspection or a reviewed upgrade.
- Binary archives include a Python 3.11+ standalone skill installer. It previews by default,
  requires an explicitly selected compatible CLI and trusted whole-ZIP digest, validates
  every resource/checksum and refuses overwrites, unsafe archives and symlink ancestors.
  `--host codex` serves Codex CLI and its extension in VS Code or Cursor; `--host cursor`
  selects the native Cursor Agent loader. Installing files does not prove model execution.
- Bundled SQLite is updated through rusqlite 0.41.0 / libsqlite3-sys 0.39.1 for
  RUSTSEC-2026-0459. Dependency notices and lock digests are refreshed; no state schema
  migration or frontend dependency update is introduced.
- Current installation guides and crate READMEs are generated from the release metadata.
  All seven project READMEs (root translations, four crates and Workbench) are registered
  in that generation; an unregistered app/crate README blocks the documentation check.
  A voluntary GitHub feedback form captures difficulties with setup, arguments, blockers
  and continuity. No automatic feedback upload or telemetry is added.

Validation uses disposable publication transports and project fixtures. A separately
captured public v1.7.1 catalog installation and Codex CLI greeting Feature establish their
original bounded scope; they are not new 1.8.0 IDE or marketplace acceptance. Manifest and
state contracts remain schema 1. Native Linux acceptance is deferred; Windows and Claude
remain experimental. Previous published versions and artifacts remain unchanged.

Candidate assets cover macOS ARM64 binaries, four source crates, source/site-preview
archives, standalone skill/plugin ZIPs, verification metadata and checksums. Website
deployment, global CLI upgrades, directory submissions and publication are separate actions.
