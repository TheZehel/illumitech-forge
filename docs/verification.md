# Verification ledger

Snapshot: 2026-10-28. This ledger distinguishes verified implementation from operational acceptance.

| Gate | Result | Evidence |
|---|---|---|
| Rust formatting | passed | cargo fmt --check |
| Clippy warnings denied | passed | workspace, all targets |
| Automated tests | passed | 26 tests; manifest, false acceptance, crash/orphan, locks, timeout, output, cancellation, human review, pause/resume |
| Documentation build | passed before dependency refresh; repeat required | cargo doc --workspace --no-deps |
| Spec validation | passed | 12 specs, 24 permanent task IDs |
| Dependency advisories | passed | OSV queried 211 locked registry packages; zero findings after replacing unsafe-libyaml and updating SHA-2 |
| Third-party notices | passed | 211 dependencies inventoried; no missing notices |
| Secret scan | passed | gitleaks, redacted output, no findings |
| Legacy bootstrap | passed (planning only) | isolated Loop worktree, inspect + dry-run; 52 Loop tests |
| Codex baseline | passed | real native CLI; implementation and read-only review, independent external gate |
| Codex toolkit | running | identical disposable fixture with shared instructions |
| Claude native | blocked | existing API-key auth returned insufficient credit; existing model configuration incompatible. User requested Codex-first validation |
| ai-memory backup | passed | online snapshot + complete frozen copy, private external storage, SHA-256 |
| ai-memory rehearsal | incomplete | original IDs/content preserved; new auto-profile behavior discovered and disabled for a fresh rehearsal |
| ai-memory production cutover | not_run | active service remains 1.17.3 until rehearsal passes |
| CI macOS/Linux | not_run | workflow prepared; pending remote execution |
| Private release v0.1.0 | blocked | integrated acceptance, memory and human review gates remain |

The first Codex run exposed a fixture issue: Python import created __pycache__. Forge correctly
blocked acceptance because the evaluator changed the worktree, even though its exit code was 0.
The evaluator now disables bytecode generation; explicit reconciliation invalidated previous evidence.

Core review: the user explicitly approved the Rust core for real-client acceptance in this session.
This is not approval to waive the pending Claude or memory acceptance requirements.

Private operational artifacts, native session identifiers, personal paths and raw output stay outside
this repository. The product source contains no production memory corpus or credentials.
