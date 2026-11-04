# Contract v1

## First-run additions (FORGE-040, 1.1.1)

`demo create DIRECTORY` prepares a NEW disposable directory; it does not execute a release.
Its schema-1 response includes `manifest`, `repo`, `state_dir`, `executed: false`,
`native_inference_calls: 0` and explicit `commands` argv arrays for inspect/dry-run/run/report.
Only Git is required. The manifest pins the external running binary as evaluator input;
replacement of that binary invalidates the example. Existing destinations and symlinks are
refused. Ambient Git routing overrides are rejected before creating files. Synthetic Git
hooks/templates are empty, signing/fsmonitor disabled locally; no global configuration changes.

`init PATH` now preserves existing memory routing and creates none by default. Explicit
`--memory-workspace ID` creates routing only if absent, or reuses exactly matching content;
different existing configuration is preserved and rejected. `memory_config` is an additive
status string: `not_requested`, `created`, or `preserved`. Workspace IDs follow `valid_id`.

`installation doctor` retains legacy fields and adds `path_diagnostics` with its own
`schema_version: 1`. Both executable names report candidates and the first eligible PATH
entry. Candidate versions remain null; only the running binary's version is known. No
candidate is executed. At most 128 PATH directories are inspected; truncation is disclosed.
Shell aliases/functions, command caches and ACLs are outside this diagnostic.

These additions are included in 1.1.1 and are not available in 1.1.0.
Status/report exit codes now prefer `effective_status`: stale evidence returns 2 even when
the historical `run.status` snapshot says `passed`. The JSON snapshot and read-only behavior
remain unchanged; automation must inspect current evidence, not only historical outcome.

## Manifest and operational contracts

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
Inspect and dry-run only validate and describe; existing duration history may be read through a
read-only store. They never create or migrate operational state.

SQLite schema 1 stores run snapshots, ordered events, control requests and revision-bound approvals.
State snapshot and event append commit together. Fingerprints include Git HEAD, staged diff and
tracked/untracked (nonignored) file bytes, mode and symlink targets. Ignored build outputs do not
invalidate source revision; explicit artifact hashes are checked at evaluation. Large files above
64 MiB block fingerprinting. Git submodules need separate repo entries for independent evaluation.

State is operator-owned. Run with permissions that prevent executors from writing state and
external evaluators; an agent with unrestricted OS access is outside the threat model.
Each task log is redacted and bounded to 1 MiB per stream. Every attempt is retained under
`logs/RELEASE/generation-N/TASK-ATTEMPT.json`; reconciliation preserves earlier generations and
the complete event/usage journal. Legacy log paths remain untouched. Keep state private and rotate
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
contract plus the explicit M08 CI side-effect authorization; actions-off blocks remote writes. `release draft-pr MANIFEST` invokes it explicitly even when the daemon is disabled.
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

## Duration estimates (FORGE-020)

`release inspect`, `release run --dry-run`, execution results, `status` and `report` add
`duration_estimate` to the schema-version-1 envelope. The estimate has its own `schema_version: 1`,
`status` (`preliminary`, `estimated`, `unavailable`), `basis`, `confidence`, `remaining_seconds`
(`{lower,upper}` in seconds or null with `unavailable_reason`), `assumptions`, `counts`,
`manifest_hash`, `execution_context_hash`, `configuration_observability`, `phase` and
`generated_at_epoch_ms`. Counts include per-task gate invocations plus final re-evaluation;
execution defaults to serial; opt-in parallel scheduling is defined in [M07](m07-operating-contracts.md). Timeout is never an expected duration. Native client/model configuration
and permissions are unchanged; estimation performs no client/network call or API fallback.

`observed_active_ms` is the checkpointed/provisionally observed active subtotal;
`observed_active_seconds` is null when the total is unknown (legacy/incomplete observation).
`in_progress_elapsed_seconds` identifies the provisional live interval. `waiting_for`,
`conditional` and `wall_clock_eta: null` prevent approval/pause/blocking from becoming a calendar ETA.
Status/report refresh projections without writing. Absent state is not created. Incompatible,
corrupt or missing history is disclosed; no synthetic samples are substituted.

Run snapshots gain an optional `duration` field (serde default for legacy snapshots). It retains
the initial/current forecasts, observed total, current/last interval, phase progress and generation.
SQLite schema 1 gains additive measurement/forecast tables and indexes on writable open only.
Run snapshot, event, measurement and forecast revision commit atomically. Finalized measurements
are immutable across retries. Reconciliation archives the previous run in its event and keeps
old measurement generations. The existing permission, lock, evaluator and reconciliation checks
remain authoritative. Never use an older binary to manage the new duration records.

Measurement matching hashes the canonical root and task/executor/spec/gate contract, declared
model and permission, platform/architecture and Forge version. Native effective configuration is
only partially observable; confidence stays low even with matching declared settings. Queries
read at most 200 measurements of at most 64 KiB each, 100 forecast revisions, and 1,000 events.
Legacy/corrupt histories cannot establish timing compatibility or successful completion.

Report adds `duration_revisions` and `duration_comparison`: immutable initial estimate, observed
active time, operational outcome, signed midpoint error and interval coverage for fully measured
completed runs. Partial/failed/cancelled runs have no completion coverage; no accuracy target is
invented. See [the duration guide](duration-estimates.md) for heuristic ranges, retry accounting,
calibration limits and clock/crash behavior. Duration data is informational and never acceptance evidence.


## Native reliability and token context (0.4 local increment)

[Protocol reliability](execution-reliability.md) defines explicit transient retries, permanent
blocks and complete recovery journals with bounded report projections. Permission/authentication,
contract and evidence failures never cause blind native retries or transfer approvals.
Declared command executors preserve their existing bounded gate-failure retry contract.

[Token measurement and bounded context](token-economy.md) defines optional native counters on
`status`/`report`, durable per-call observations, session/turn deduplication and unknown coverage.
`report --limit N --offset N` pages events/calls; totals cover the full release journal.

`context plan MANIFEST --task ID --budget-tokens N` is a read-only deterministic preview.
Manifest and task `context` accept `profile: full|bounded`, optional `budget_tokens` (required
for bounded) and repo-relative `sources`. Task policy replaces manifest policy; full is default.
No instruction, acceptance, native model, account or permission is removed to meet a prompt budget.
Mandatory oversize blocks execution. No inference call is added to optimize context.

## M06 additions

See [M06 operating contracts](m06-operating-contracts.md) for strict manifest/task `budget`,
opt-in gate `cache`, versioned templates, and authenticated asynchronous workbench controls.
Existing manifests preserve their behavior through additive defaults; human review remains
revision-bound and the legacy dashboard stays read-only.

## Parallel worktrees and independent review (FORGE-030/031)

See [M07 operating contracts](m07-operating-contracts.md) for `execution.max_parallel`,
root ownership, shared budgets, revision-bound review, final gate cancellation and
verified diagnostic references. These are additive, opt-in execution changes; review
never grants release acceptance.

## Selected issue and publication safety (FORGE-032)

See [M08 operating contracts](m08-operating-contracts.md) for the additive strict `source`
schema, selected authenticated gh/offline snapshot import, content-hash binding, traceability
and publication preview. Import is preview-first and never grants executable authority.
`budget.actions:off` blocks staging/commit and remote writes that could trigger CI; a separate
`actions:on` plus `automation.publish.ci_authorized:true` contract permits possible CI side
effects without attesting provider quota. Tasks cannot enable CI. Readback after every draft
create and the existing publication journal recover uncertain requests without duplication;
remote divergence blocks rather than overwrites. Declared human gates remain revision-bound;
imported candidates include them. The UI can
inspect a publishing manifest but cannot execute it or publish.

See [M09 safe installation and recovery](m09-operating-contracts.md) for explicit-prefix verified updates, consistent SQLite snapshots, fresh-target restore and platform limits.

## Workflow, comparison and SARIF contracts (candidate 1.2.0)

New contracts each use schema 1; manifest/envelope schema 1 and existing commands are unchanged.
Workflow prepare is preview-first; --apply saves external candidates/check sources only. Start requires
an exact scope digest, materializes create-only specs and invokes the same engine inspection/dry-run
checks before execution. A transactional metadata/event journal records consent and checkpoints.
The initial execution revision is checked under the engine worktree lock. Neither consent nor a
native tool approval grants external gate or human acceptance. Requests cannot enable publication.

Successful planning receipts from the owning native server are linked into the execution token journal; failed or unlinked planning remains outside it and is disclosed.
Checks bind exact argv, optional task_ids, frozen external files and optional nonzero before_exit.
Bug recipes require a frozen failing regression. Dependency recipes require a SARIF rescan plus
project tests; Semgrep rescan requires --error. Native configuration is preserved. Context defaults
to bounded/16384 only for new workflows; mandatory oversize blocks instead of dropping instructions.

Candidates are immutable. A changed source/contract before start requires a fresh reviewed workflow
ID; existing task IDs/evidence are preserved rather than rewritten. Interrupted preparation/baselines
are diagnosed without replay. Explicit resume/reconciliation retains the existing engine controls.
Scope metadata and events are extensions of SQLite schema 1, not transferable human approvals.

Comparison is read-only and uses complete counters plus operator-declared evaluation_context.
Missing/mismatched experiment context or incomplete/nonaccepted samples cannot produce a savings
figure. It never authenticates a native model/account from caller-provided JSON, converts tokens to
currency, or claims generalized savings. SARIF imports accept 2.1.0, reject remote/traversal locations
and unresolved bases, preserve scanner errors and never execute findings/fixes or mark them resolved.

## FORGE-053 — Installation diagnostics in doctor

`doctor` adds `installation`, identical to the read-only `installation doctor` response.
Each PATH command adds `status` (`resolved`, `attention`, `not_found`, `incomplete`), `distinct_targets`
and `next_action`. Canonical targets deduplicate symlinks; the two command names are
not compared as if they must be the same file. Candidate versions remain unknown.
Only the running version is reported; existing processes must be reopened after updating.
The main doctor still performs its existing bounded client capability checks.

FORGE-053 also adds per-command `coverage` (`complete` or `partial`). The invoked command name
comes from argv[0], before resolving symlinks. A truncated PATH scan cannot prove absence or
uniqueness: status is `incomplete` unless a conflict already warrants `attention`.

## FORGE-054 — Release version policy

`release version --changes FILE [--target VERSION]` reads a bounded strict JSON change set
and returns the highest declared compatibility impact, proposed version/tag and input digest.
Fix is patch; feature/deprecation minor; breaking major; docs/internal alone produce no release.
The optional exact target check returns blocked/exit 2 on mismatch. A passing result grants no
publication authority and creates no state or configuration. See [policy](release-version-policy.md).
