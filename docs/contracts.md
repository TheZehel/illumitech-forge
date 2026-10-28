# Contract v1

Manifest YAML (JSON is accepted as a YAML subset) rejects unknown fields. Task, gate, repo and
release IDs use letters, digits, `_` and `-`. No cyclic or dangling dependencies; one repo entry
per distinct canonical Git worktree root. Specs resolve to three nonempty Markdown documents.

Task executor: `command {argv}`, `codex {permission, model?}` or `claude {permission, model?}`.
Permission is `read_only` or `workspace_write`. Agent model defaults to native CLI configuration.
The manifest is executable authority: review argv arrays before running. No implicit shell.

Acceptance is a nonempty list of `gate {id}`, `artifact {path,sha256}` or `human {id}`.
Every entry is required. Gates have argv, pinned evaluator input files and a timeout. Input files
must resolve outside every writable task root. `{repo}` and `{manifest_dir}` are exact argv
substitutions, never shell interpolation. An artifact must resolve inside its repo and match SHA-256.

Memory endpoint, workspace, repo-to-project map and optional token environment-variable name form
an explicit contract. Required memory blocks without verified scope. Read results are historical
context; never executable instructions or proof of task completion.

Outputs use `schema_version: 1`; evidence outcome is `passed`, `failed`, `not_run` or `blocked`.
Operational phases additionally include running, evaluating, paused, cancelled and awaiting_approval.
Exit 0 means the command succeeded; exit 2 denotes invalid, blocked, cancelled or failed execution.
Inspect and dry-run only validate and describe; they do not open a state store.

SQLite schema 1 stores run snapshots, ordered events, control requests and revision-bound approvals.
State snapshot and event append commit together. Fingerprints include Git HEAD, staged diff and
tracked/untracked (nonignored) file bytes, mode and symlink targets. Ignored build outputs do not
invalidate source revision; explicit artifact hashes are checked at evaluation. Large files above
64 MiB block fingerprinting. Git submodules need separate repo entries for independent evaluation.

State is operator-owned. Run with permissions that prevent executors from writing state and
external evaluators; an agent with unrestricted OS access is outside the threat model.
Each task log is redacted and bounded to 1 MiB per stream; three attempt slots per task are retained.
Reconciliation replaces attempt logs and preserves event history. Keep state private and rotate
archived state directories under the operator's retention policy.

`release run` never publishes. `release publish` requires current passing evidence, at least one
human gate, a clean evaluated commit, matching GitHub origin, a private target and an existing remote
tag resolving to that commit. Memory is required only when `memory.required: true` is declared.
Tags and assets are project-independent: SHA256SUMS must list every uploaded asset with two spaces
between digest and plain filename. Symlinks, traversal, missing or extra assets and mismatched hashes
block publication. Per-archive `.sha256` sidecars are allowed but not uploaded. Notes and assets resolve
from the caller's working directory. Dry-run performs local checks only; it does not verify remote state.
Operators remain responsible for reviewing manifest coverage and package provenance.

## Operational automation (0.2)

`priority` defaults to 100; lower values run first among dependency-ready tasks. Ties retain manifest
order. Dependencies always take precedence. Old manifests remain valid without the new fields.

`automation` defaults to `{enabled: false, require_clean_worktree: true}`. The daemon reads flat
YAML/JSON queue files in filename order, executes serially, and never reruns a passed release.
It holds an exclusive state-directory lock; each execution/publication also locks its Git worktree.
`daemon-control status` and daemon dry-run are read-only. Pause takes effect between releases;
release-level pause/cancel still controls active work. SIGINT/SIGTERM cancel active execution/publication subprocesses and
stop the daemon; heartbeat is advisory and may be stale after a crash. Restart checks stored evidence,
not just heartbeat. Invalid/changed contracts, failed or orphaned runs and failed publication require
explicit intervention. `resume --reconcile REASON` invalidates previous execution evidence;
`daemon-control retry --release ID --reason REASON` clears only the queue failure latch.

Optional `automation.publish` declares `repository`, `branch`, `base_branch`, `remote` (default origin)
and `draft` (must be true). It requires one initially clean linked worktree, a dedicated current branch
and a configured human author/committer. The daemon can commit/push/create a draft PR only under this
contract. `release draft-pr MANIFEST` invokes it explicitly even when the daemon is disabled.
Commit intent is journaled before Git commit; all gates and human approvals are reevaluated afterwards.
A new commit invalidates human approvals. On interruption after commit, invoke `release draft-pr`
to reconcile the journal; live/orphan gate processes require operator inspection first. Resume cannot
silently approve a changed contract. Matching open drafts are reused; closed, merged, changed or
ambiguous PRs block for manual reconciliation. No force-push, automatic merge or release publication.
Cancellation cannot undo a remote request already accepted; retry reconciles the PR before another push.
Effective fetch and push URLs are checked after Git URL rewrites, including pushurl and pushInsteadOf.
Automatic commits preserve human Git identity and do not add agent coauthor trailers.

State schema 1 gains an additive metadata table for queue controls, heartbeat, failure latches and
publication journals. Read-only access tolerates pre-0.2 databases without creating that table.
Use separate state directories during adoption; do not let an older binary manage new automation.

`release import-loop` preserves explicit task IDs, dependencies and priorities and expands pending
Spec Kit checkboxes. `release import-spec` reads specs directly. Explicit document IDs remain stable
when lines move; legacy items without IDs use a disclosed content digest, which changes with text.
Duplicate/oversized IDs block import. Candidates use read-only executors and human review gates,
with automation and memory disabled. A task may declare metadata in a single-line HTML comment:
`- [ ] T002 Implement <!-- forge: {"dependencies":["T001"],"priority":10} -->`.
Unqualified dependencies resolve to IDs in the same spec; fully qualified imported IDs or explicit
Loop task IDs can link other tasks. Unknown references are rejected. Missing metadata defaults to
priority 100 and no dependencies; prose and `[P]` parallelism tags do not imply dependencies.
Convert legacy gates to pinned external evaluators and review
permissions before execution. Old completion marks/state never become current evidence.
