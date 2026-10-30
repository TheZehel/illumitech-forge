<p align="center"><img src="docs/assets/logo.webp" width="108" alt="Illumitech logo"></p>

# Illumitech Forge

**From contract to evidence.** A Rust CLI for coordinating releases with Codex and Claude Code.
An agent saying “done” never proves that a task passed.
**v0.3.0 candidate: local dashboard, per-user services, optional tracing and public distribution.**
Codex is the validated client; Claude remains experimental. [Approved scope](docs/adr/0001-first-delivery-scope.md).

[Português](README.md) · [Contracts](docs/contracts.md) · [Expansion evidence](docs/expansion-runtime-acceptance.json)

![An orange engineered cube passes three checkpoints toward a petrol-green structure](docs/assets/cover.webp)

Initially private, MIT-licensed from the first version. Public visibility is a later explicit decision.
[Version 0.1.0](https://github.com/TheZehel/illumitech-forge/releases/tag/v0.1.0) was published after human approval.
[Version 0.2.0](https://github.com/TheZehel/illumitech-forge/releases/tag/v0.2.0) is also published.
The current candidate has separate [distribution gates](docs/public-distribution.md).

## Install and try

Download the platform archive from the private release; the binary needs no Rust toolchain.
The [first-run guide](docs/quickstart.md) covers checksums and bundled examples with and without
Codex. Examples require Git and Python 3.

Supports macOS ARM64 and Linux x86_64. Rust 1.94.0 and Git are required for source builds.
The CI Linux binary requires glibc 2.39 or newer (Ubuntu 24.04); other distributions can build from source.
Windows has no operational support yet. Install and authenticate agent CLIs separately.

```sh
cargo build --release --locked
./target/release/illumitech-forge doctor
python3 scripts/demo.py /tmp/forge-demo
./target/release/illumitech-forge --state-dir /tmp/forge-demo/state release inspect /tmp/forge-demo/release.yaml
./target/release/illumitech-forge --state-dir /tmp/forge-demo/state release run /tmp/forge-demo/release.yaml --dry-run
./target/release/illumitech-forge --state-dir /tmp/forge-demo/state release run /tmp/forge-demo/release.yaml
```

The demo uses a disposable Git repository and an external evaluator; it calls no LLM.
Private release archives include both platform binaries, SHA-256 checksums and third-party notices.
Registry packages are prepared; upload and public visibility require the evaluated revision's gates.
After publication, install with `cargo install illumitech-forge-cli --version 0.3.0 --locked`.

## Architecture and workflow

Specs and ADRs own intent; YAML manifests own execution contracts; SQLite owns operational state;
Git owns source; ai-memory owns contextual continuity. Four crates separate domain, adapters,
catalog and CLI. State and evaluators live outside agent-writable repositories. OS locks enforce
one Forge writer per worktree. This is a trusted local orchestrator, not an OS security boundary.

Create specs with permanent IDs, inspect the manifest, review its dry-run, execute, evaluate and
explicitly approve the evaluated revision. `pause`, `resume`, `cancel`, `status` and `report` control
persistent runs. Publication is a separate `release publish` action. `--json` uses schema version 1;
blocked outcomes return exit code 2. Read-only commands create no execution state.

Default limits are 30 minutes per task, 10 minutes per gate and three total execution attempts.
Permission, contract, human approval and mandatory-memory failures never retry automatically.
Process groups receive TERM then KILL on cancellation or timeout; output is bounded and redacted.
Changed manifests, worktrees or orphaned tasks require explicit reconciliation. Approval is bound
to the evaluated fingerprint and cannot survive a material revision change.

## Memory and native clients

`memory doctor` verifies MCP capability and explicit workspace/project scope; `memory query` returns
bounded sourced context. Configure `memory.required: true` to fail closed; optional mode records
degradation. Memory is optional for execution and publication. HTTPS is required except on loopback. Tokens are environment-variable references.
Forge neither accesses the memory database nor writes durable pages or duplicates lifecycle hooks.
Shared procedures live in `skills/core/SKILL.md`.

Codex and Claude preserve native authentication, configuration and default model. A model is passed
only when explicitly selected in the manifest. Codex uses a declared sandbox; Claude uses `dontAsk`
and existing permissions. Grant project-scoped `Edit` rules before write tasks. There is no silent
fallback to paid APIs. Structured client results are necessary but never sufficient acceptance.

## Toolkit, quality and adoption

`tools list/doctor/plan/apply` supports selective, idempotent local recipes. It never runs remote
installers. Availability, installed recipe, executable discovery and functional validation remain
separate. Rust, Node/TypeScript and generic profiles preserve existing conventions. RTK/Graphify
pilots and optional workflow styles do not impose universal file-size or prose rules.

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --no-deps --locked
python3 scripts/licenses.py
```

`eval run` executes fixture manifests through the same engine. Tests cover false completion, stale
approval, required memory, locks, timeout, output bounds, cancellation, pause/resume and repeated
setup. Real clients and cross-platform CI remain separate acceptance evidence in the ledger.
`release import-loop` expands pending Spec Kit tasks into reviewable candidates with stable IDs;
`release import-spec` imports specs directly. Historical completion is never execution evidence.
The Python Loop remains operational while projects migrate gradually.

The 19 specs cover the delivery contract. The opt-in foreground daemon supports persistent queue
controls and failure latches. Optional draft PR automation requires an isolated worktree and human
Git identity, reevaluates gates after commit and recovers without duplicate publication. It never merges
or publishes releases automatically. See [operations and adoption limits](docs/operational-parity.md).
The [local dashboard](docs/dashboard.md) reads releases, tasks, gates and events without running
agents. [Per-user services](docs/user-services.md) have explicit preview, application and control;
the macOS lifecycle passed an isolated real pilot. Linux/Windows service lifecycle gates remain
separate. [Tracing](docs/observability.md) is off by default, content-free and fail-open.

The operator's ai-memory 3.2.9 runtime was migrated to native launchd; the dedicated Colima VM
is stopped and rollback storage is retained. Memory remains optional for Forge users.
[Migration tooling](docs/native-memory-migration.md) has offline safety tests; its generic workflow
and the separately recorded operator migration have distinct evidence. Public visibility and
registry uploads remain explicit operations. No paid benchmarks or official Harness² framework are bundled.

Illumitech · MIT. See [THIRD_PARTY_NOTICES](THIRD_PARTY_NOTICES). No unmeasured performance badges.

Windows 11 x86_64 / PowerShell 7 has an unreleased portability candidate with native CI and ZIP packaging. Native execution and authenticated pilot acceptance remain pending; published v0.2.0 artifacts are unchanged. See the [Windows inventory and gates](docs/windows-portability.md).
