<p align="center"><img src="docs/assets/logo.webp" width="108" alt="Illumitech logo"></p>

# Illumitech Forge

**From contract to evidence.** Turn a request into specs and tasks, execute with the native
client, and inspect verifiable results through the CLI or Illumitech Workbench.

**Illumitech Forge 1.4.0.** Installation diagnostics now preserve invoked aliases and disclose incomplete PATH scans. The new `release version` command computes and validates Semantic Versioning from reviewed changes. See the [release notes](docs/release-v1.4.0.md) and [version policy](docs/release-version-policy.md). Availability and checksums are recorded in [GitHub releases](https://github.com/TheZehel/illumitech-forge/releases); building this tree does not update your installed binary.

![An orange component passes checkpoints toward a petrol-green structure on graphite](docs/assets/cover.webp)

The **1.1** series provides the native Rust `forge` command and a terminal experience with
ASCII identity, Markdown, command menu, visual help and searchable session history.
See [1.2.0 release notes](docs/release-v1.2.0.md) for validation and operational limits.

## Features

Duration ranges/history; measured tokens and bounded context; local native chat and project
sessions; guided specs and structured DAG proposals; versioned templates; execution controls,
revision-bound approval, budgets and conservative gate caching; parallel existing worktrees;
independent read-only review; selected issue import and explicitly authorized draft PRs;
installation diagnostics, verified local updates, SQLite snapshots and fresh-target restore.

Codex uses the configured native account/model. Managed ChatGPT App Server protocol 0.166.0
is the reference for Workbench. Claude is experimental/deferred. Free chat is read-only;
implementation runs under a reviewed manifest. Agent messages never satisfy acceptance gates.
No paid API fallback or permission bypass is introduced.

## Start with the published release

```sh
cargo install illumitech-forge-cli --version 1.4.0 --locked
forge --version
forge
```

Run `forge` inside an existing Git project. Install and authenticate the native Codex CLI
separately for conversations; its account, model and permissions are preserved. Opening the
interface needs no `init`, ai-memory, Python or Node. Free chat is read-only; implementation
execution requires a reviewed manifest.

Cargo needs Rust 1.94.0. The [public macOS ARM64 release archive](https://github.com/TheZehel/illumitech-forge/releases/tag/v1.2.0)
needs no Rust compiler. The GitHub repository and crate source are public under MIT.
See the [quickstart](docs/quickstart.md) for installation, version/PATH checks and Workbench.

## First-run foundation

- `forge demo create DIRECTORY`: native command/gate/state example without an agent, account
  or Python. See the [walkthrough](docs/quickstart.md#demonstracao-sem-agente).
- `init` creates optional routing only with `--memory-workspace ID`, preserving existing files.
- `installation doctor` inspects both executable names without running unknown candidates.
- `status` and `report` return 2 for stale evidence, preserving historical JSON snapshots.

Build with `cargo build --locked -p illumitech-forge-cli --bins`. UI assets are embedded;
frontend development uses Node 24 LTS and `npm ci --prefix apps/forge-ui --ignore-scripts`.
Building source never replaces installed executables or changes PATH.

## Workflow and boundaries

Request → specs → validated proposal → inspect/dry-run → execution → external gates →
declared human acceptance. Proposals do not authorize execution/publication. Keep manifests,
pinned evaluators and operational state outside agent-writable roots. One writer per root.
Specs/ADRs own intent, manifests execution, SQLite operations and Git source. **ai-memory is
optional**, accessed through MCP. Required mode fails closed only when explicitly selected.
Existing native configuration and the legacy Loop remain available; specialized skills
complement the engine.

Token counters cover journaled invocations, not all account activity. Cached input is part
of input; unknown usage remains unknown. A native invocation is not a provider request count.
Configured hooks/MCP/background work can fall outside this journal. Estimates are uncertain
active-work ranges, not deadlines. Neither savings percentages nor subscription prices are
promised. Gate caching safely misses for unobservable inputs, dependency symlinks or oversized
trees; no quality-cache saving was established for this program's own repository.

Earlier checkpoints have macOS native pilots and a Linux ARM64 VM/container baseline.
Final-source acceptance is separate. Linux native sandbox preflight was blocked in this lab;
Windows/PowerShell is experimental without a native pilot. Fixtures do not prove provider or
platform acceptance. See the [support/evidence guide](docs/v1-candidate.md).

Actions, push/merge, releases, registry upload, visibility changes, global installation and
service activation require separate explicit authorization. Actions default off; a skip-ci
marker cannot guarantee that remote events will not trigger workflows.

[Contracts](docs/contracts.md) · [Workbench](docs/workbench.md) · [Economy](docs/token-economy.md) ·
[Duration](docs/duration-estimates.md) · [M06](docs/m06-operating-contracts.md) ·
[Parallel/review](docs/m07-operating-contracts.md) · [Issue/PR](docs/m08-operating-contracts.md) ·
[Update/recovery](docs/m09-operating-contracts.md) · [Distribution](docs/public-distribution.md).

Copyright Illumitech, [MIT](LICENSE). Preserve [third-party notices](THIRD_PARTY_NOTICES),
[dependency inventory](docs/dependencies.json) and [asset provenance](docs/assets/provenance.json).
Commits and releases use the configured human identity, with no agent authorship attribution.

## New in 1.2.0

Integrated workflows, scope consent, external verification, SARIF import and delivery comparisons
are available in 1.2.0. See [workflow guide](docs/guides/workflows.en.md),
[economy](docs/guides/economy.en.md) and [implementation limits](docs/growth-candidate.md).
The GitHub repository is public as of 2026-11-02. Source and release verification identify the evaluated revision.
The PT/EN website is a local preview; no domain, telemetry or deployment is enabled.
