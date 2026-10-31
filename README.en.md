<p align="center"><img src="docs/assets/logo.webp" width="108" alt="Illumitech logo"></p>

# Illumitech Forge

**From contract to evidence.** Turn a request into specs and tasks, execute with the native
client, and inspect verifiable results through the CLI or Illumitech Workbench.

**This tree is an unpublished local 1.0.0 candidate.** The crates.io version is 0.3.0.
Building this tree does not replace an installed binary. Independent operator evidence owns
final operational acceptance. [Português](README.md) · [Candidate guide](docs/v1-candidate.md).

![An orange component passes checkpoints toward a petrol-green structure on graphite](docs/assets/cover.webp)

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

## Local build

Rust 1.94.0 and Git are required to build. Git/Python 3 support the examples; the Codex CLI
is authenticated separately. UI assets are embedded; using the binary needs no Node runtime.
Frontend development uses Node 24 LTS and the committed npm lockfile.

```sh
cargo build --release --locked
./target/release/illumitech-forge --version
./target/release/illumitech-forge --json contracts
FORGE_OPS="$(mktemp -d)"
./target/release/illumitech-forge --state-dir "$FORGE_OPS/state" workbench --root "$PWD" --port 0
```

Open the private local URL emitted by the foreground server. It binds only to loopback.
For a no-inference demo, run `python3 scripts/demo.py EXTERNAL_DEMO_DIR`, then inspect,
dry-run and run its release with an explicit external `--state-dir`.
The published version can be installed using
`cargo install illumitech-forge-cli --version 0.3.0 --locked`; 1.0.0 has not been uploaded.

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
