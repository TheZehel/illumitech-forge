# Verification ledger

Snapshot: 2026-10-28. This ledger distinguishes verified implementation from operational acceptance.

| Gate | Result | Evidence |
|---|---|---|
| Rust formatting | passed | cargo fmt --check |
| Clippy warnings denied | passed | workspace, all targets |
| Automated tests | passed | 26 tests; manifest, false acceptance, crash/orphan, locks, timeout, output, cancellation, human review, pause/resume |
| Documentation build | passed | cargo doc --workspace --no-deps |
| Spec validation | passed | 12 specs, 24 permanent task IDs |
| Dependency advisories | passed | OSV queried 211 locked registry packages; zero findings after replacing unsafe-libyaml and updating SHA-2 |
| Third-party notices | passed | 211 dependencies inventoried; no missing notices |
| Secret scan | passed | gitleaks, redacted output, no findings |
| Legacy bootstrap | passed (planning only) | isolated Loop worktree, inspect + dry-run; 52 Loop tests |
| Codex baseline | passed | real native CLI; implementation and read-only review, independent external gate |
| Codex toolkit | passed | real native implementation + read-only review; clarified review prompt shared by both arms |
| Claude native | blocked | existing API-key auth returned insufficient credit; existing model configuration incompatible. User requested Codex-first validation |
| ai-memory backup | passed | online snapshot + complete frozen copy, private external storage, SHA-256 |
| ai-memory rehearsal | passed | clean 3.2.9 copy: integrity, all IDs, 40 projects, 615 pages and content preserved; old-version restore validated |
| ai-memory production cutover | passed; client acceptance partial | 3.2.9 active on a separate volume; final frozen backup; old volume and image retained |
| CI macOS/Linux | pending | workflow prepared; GitHub workflow authorization granted; remote run pending |
| Private release v0.1.0 | blocked | integrated acceptance, memory and human review gates remain |

The first Codex run exposed a fixture issue: Python import created __pycache__. Forge correctly
blocked acceptance because the evaluator changed the worktree, even though its exit code was 0.
The evaluator now disables bytecode generation; explicit reconciliation invalidated previous evidence.

Core review: the user explicitly approved the Rust core for real-client acceptance in this session.
This is not approval to waive the pending Claude or memory acceptance requirements.

Private operational artifacts, native session identifiers, personal paths and raw output stay outside
this repository. The product source contains no production memory corpus or credentials.

Codex acceptance details: [client scenarios](codex-acceptance.json) and
[mandatory memory](codex-memory-acceptance.json). The fresh session called the real MCP tools and
recovered earlier sessions in the expected project scope; new lifecycle capture is pending native
hook trust. Both client configurations passed idempotent hook installation, preservation of unrelated
settings and one ai-memory handler per event. Claude was not run again after the user's deferral.

The first production cutover rolled back automatically because the operator probe used an unavailable
HTTP route. The corrected MCP handshake passed after a new final backup and migration to another
separate volume. The earlier migrated volume was never opened by the old binary.

The macOS ARM64 archive has been built and smoke-tested locally. Linux x86_64 and native CI remain
pending. [Tool pilots](tool-pilots.json) cover only small disposable fixtures, with no performance claim.

Release remains blocked until Claude acceptance, lifecycle capture/continuity, native CI and final
human release review pass. No v0.1.0 tag or GitHub release has been published.
