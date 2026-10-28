# Verification ledger

Snapshot: 2026-10-28. Implementation evidence and integrated release acceptance are separate.

| Gate | Result | Evidence |
|---|---|---|
| Formatting, Clippy, documentation | passed | local checks and native CI; warnings denied |
| Automated Forge tests | passed | 27 per platform, including false acceptance, crash/orphan, locks, timeout, bounded output, cancellation, human approval and pause/resume |
| Spec validation | passed | 12 specs, 24 permanent task IDs |
| Dependency advisories | passed | OSV: 211 locked registry packages, zero findings |
| Third-party notices | passed | 211 dependencies inventoried; no missing notices |
| Secret scan | passed | gitleaks, redacted output, no findings |
| Legacy bootstrap/import | passed (planning and import only) | [12 read-only candidates](legacy-import.json), isolated Loop worktree, inspect/dry-run and 52 tests |
| Codex baseline/toolkit | passed | [real native implementation and read-only review](codex-acceptance.json), external pinned behavioral evaluator |
| Codex mandatory memory | passed | [fresh native MCP calls](codex-memory-acceptance.json), previous sessions recovered in illumitech/illumitech-forge |
| New lifecycle capture | passed | [three bounded real scenarios](lifecycle-acceptance.json); trusted hooks, unique lifecycle events, preserved files and no cross-project rows |
| Claude native | experimental, outside v0.1.0 acceptance | [explicit scope decision](adr/0001-first-delivery-scope.md); no further Claude runs attempted |
| ai-memory rehearsal | passed | isolated old/new copies; integrity, IDs and content preserved; old-version restore validated |
| ai-memory production upgrade | passed for Codex scope | [3.2.9 active](memory-upgrade-result.json), final backups, separate volume, retained 1.17.3 image and volume |
| ai-memory client configuration | passed | idempotent install; unrelated settings preserved; one ai-memory handler per event |
| Forge repository memory scope | passed | explicit marker, registered by real SessionStart; memory doctor verified illumitech/illumitech-forge |
| CI macOS/Linux | passed | [run 40756018143](https://github.com/TheZehel/illumitech-forge/actions/runs/40756018143), packaging commit fea9098 |
| Binary archives and checksums | passed | [distribution evidence](distribution-result.json); downloaded CI archives verified locally |
| Private repository | published | [TheZehel/illumitech-forge](https://github.com/TheZehel/illumitech-forge), MIT, private visibility verified |
| Private release v0.1.0 | awaiting final review | Codex scope approved; final candidate review and explicit publication remain; GitHub release is the publication receipt |

The first Codex evaluator imported Python and created bytecode. Forge blocked acceptance because
the evaluator changed the worktree, even though it exited 0. The corrected evaluator disables
bytecode; explicit reconciliation invalidated old evidence. The review prompt was clarified identically
in both arms after an initial toolkit timeout. This small evaluation establishes correctness and
preservation, not a performance, cost or general agent-quality benchmark.

A pinned artifact ignored by Git still requires explicit reconciliation if changed. The regression
was reproduced before correction; the 27-test suite and native CI passed afterwards.

The first production migration rolled back automatically because the operator used an unavailable
HTTP health route. The corrected MCP handshake passed after another final backup and migration to
a fresh volume. No migrated database was opened with the old binary. The final snapshot preserves
all old IDs and all 624 existing page bodies, paths and update timestamps. The OKF migration also produced 42
deterministic project bundle indexes, which the live wiki watcher indexed. Automatic profile
harvesting and scheduled improvement are disabled; no durable knowledge note was explicitly written.

[Tool pilots](tool-pilots.json) cover only disposable fixtures. RTK was not installed globally;
Graphify ran code-only in an isolated environment. No LLM benchmark or savings claim follows.

The user approved the Rust core, later authorized lifecycle capture, and explicitly selected a
Codex-focused v0.1.0 with Claude experimental (ADR 0001). All other release gates remain mandatory.
The first fresh-project capture exposed a 3.2.9 backfill/live-hook overlap and failed its external
gate. Disabling automatic startup backfill in the native client preserved trusted hook definitions.
The subsequent Loop run, Forge continuity run and entirely new disposable project all passed.
No old observations were deleted or rewritten. Daemon
mode remains a documented future extension; this version uses foreground supervision and persistent
control commands. The Linux CI binary requires glibc 2.39 (Ubuntu 24.04); source builds remain available.

Private operational artifacts, session identifiers, host paths, raw output and memory corpus remain
outside this repository. Binary inputs remain unchanged from code commit 4e4942d; packaging now also
includes the bilingual first-run guide and example generators. The distribution receipt identifies
the build and archive checksums. This ledger is the pre-publication snapshot; the GitHub release
records the final publication without modifying the approved source revision.
