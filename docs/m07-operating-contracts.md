# M07: parallel worktrees and independent native review

These are local product contracts for FORGE-030/031. External evaluator and human
acceptance remain coordinator-owned. Fixtures are protocol/control regressions;
they do not establish provider, Windows or remote acceptance.

## Scheduling and integration

```yaml
execution:
  max_parallel: 2
budget:
  max_concurrency: 2
review:
  max_iterations: 3
  max_tokens: 100000
  timeout_seconds: 600
```

All fields are optional. `execution.max_parallel` defaults to 1 and accepts 1..32.
`budget.max_concurrency` accepts 1..32 and further limits the scheduler. Inspect and
dry-run expose both the requested configuration and `effective_max_parallel`.
The serial scheduler retains its existing retries and cooperative pause behavior.

Parallel execution uses deterministic, priority-ordered DAG waves. A wave admits only
ready tasks, at most one task per canonical, non-overlapping manifest worktree. Workers
own bounded subprocess results; a single coordinator owns the SQLite connection and
transactionally journals run snapshots, process PIDs, usage and measurements. No
SQLite connection is passed to a worker. Worktree leases prevent another Forge writer
from entering the same roots. Source roots must already exist: this interface never
creates branches, commits, copies worker changes over another root, cherry-picks or merges.

Shared attempt and active-time limits apply at admission; task limits still apply.
Native tasks under a shared token ceiling serialize until usage is observed. Missing
usage blocks subsequent token-limited inference. Token counts cannot guarantee a hard
per-inference ceiling. Native subscription balance and monetary cost remain unknown.

Between waves, gates inspect the finished selected roots with no active writer. The
final sweep re-evaluates every task against the final source. An evaluator changing any
locked root invalidates its evidence. Unmerged Git entries block integration with a
bounded preview in the `integration_conflict` event. Files, index entries and unrelated
work are preserved for explicit operator resolution. There is no automatic conflict
resolution or implicit integration target.

Parallel pause/cancel stops **all** active owned process groups. A pause records `paused`,
a cancel records `cancelled`; partially executed tasks require explicit reconciliation.
Serial pause remains cooperative at task/gate boundaries. Unix SIGINT and SIGTERM
follow the bounded cancellation path through the final gate sweep. Listener handles
are aborted on every return. A crash releases OS leases but leaves interrupted tasks,
PIDs and measurements in the ledger: resume fails closed and reconciliation refuses
live recorded processes. Forge never kills an arbitrary recovered PID.

Duration forecasts simulate the actual DAG waves, root serialization and concurrency
limits instead of dividing the serial duration by a constant. Gates/final validation,
setup and retry risk remain conservatively serial. Parallel execution measures per-task
active time separately and counts session wall-active time once. Open intervals remain
incomplete after a crash; they are never converted into successful history.

## Independent review

```sh
illumitech-forge --state-dir /safe/operator-state review plan release.yaml
illumitech-forge --state-dir /safe/operator-state review run release.yaml
```

`review plan` is read-only: no database, model call or files are created. Its `review_plan`
contains native tasks, source fingerprints, schema, iteration/token limits and the
release budget. Each `review run` is one explicit corrective round. It starts a distinct
ephemeral App Server thread per selected root using the existing authenticated ChatGPT
account, configured model, read-only sandbox and no approval grants. No model or provider
override, API fallback, Claude dependency, tool approval or automatic correction occurs.
The optional `--app-server PATH` selects a trusted executable directly, without a shell,
for the same JSON-RPC bridge; omit it for normal native execution.

The reviewer receives the task contracts, current fingerprint and structured finding
schema. It can read local instructions/source through the native sandbox. Credential
paths cannot be attributed as findings. Persisted review usage has its own `_review`
task/phase and contributes to the shared release budget. Each root invocation has
a distinct admitted call ID and attempt; later roots recheck all shared counters
under the same source lease. A round does not hide multiple native calls as one attempt. Limits apply across explicit
rounds, including failed/unknown invocations; reconciliation of implementation evidence
does not erase review rounds. A single round has one active-time deadline across roots. Explicit new reviews resume
their controls under the lease, including when no implementation run exists. Pause/cancel
and SIGINT/SIGTERM request native tool interruption before bounded server/group shutdown.
Missing usage remains unknown. Cached input is a subset of input, never extra tokens.

Findings use this versioned, bounded schema (maximum 100 per result):

```json
{
  "schema_version": 1,
  "findings": [{
    "id": "issue-1",
    "severity": "error",
    "repo": "app",
    "file": "src/lib.rs",
    "line": 10,
    "fingerprint": "64-hex-current-source-fingerprint",
    "evidence_sha256": "64-hex-current-file-digest",
    "message": "A supported finding and its observed consequence"
  }]
}
```

IDs are unique per response. Severity is `info`, `warning` or `error`. Paths must be safe
repo-relative Git source inventory entries; symlinks escaping the root, ignored/private
paths, stale revisions, mismatched hashes, invalid lines and unknown fields are rejected.
Messages are bounded to 8192 bytes and redacted. A matching file hash establishes source
attribution, not the semantic correctness of the finding; external review remains required.

Source scope is HEAD/index, tracked files and nonignored untracked files in all locked
roots. After **every** native invocation, including timeout, failure and disconnect,
Forge checks all fingerprints before parsing any success claim. Changed/unobservable
scope produces `review.status: blocked`, `block_reason: read_only_violation` and
`changed_scope`, preserves modifications and stops further reviewer calls. Unsupported
or malformed output blocks. Nonempty findings block with `findings_require_resolution`.
Empty findings produce a completed review and `acceptance: unchanged`; no implementation
run, gate or human approval is fabricated or modified.

The state metadata retains each round and its source, usage, call ID and configuration
returned by the native thread. `source_current` in later projections explicitly exposes
staleness. Historical review results remain advisory even when source is current.

## Gate diagnostics and UI

Every launched gate retains a bounded, redacted `ProcessResult` JSON outside Git.
`Evidence.detail` remains JSON with compatible cache/result fields and adds
`log: {path, sha256}`. Paths are relative to the state directory and identify release,
generation, initial/final phase, task, gate, attempt and measurement sequence. Spawn
errors also retain a blocked diagnostic. Failed subprocess status cannot become success
through agent text. Sensitive named object/array values are replaced as whole values,
while JSON envelope, IDs, safe output and failure information remain parseable.

Cache hits keep the verified diagnostic reference and original provenance. Missing,
corrupt or escaping diagnostic files cause a miss. Status/report expose bounded safe
stdout/stderr previews and references; no arbitrary filesystem read endpoint is added.

Workbench startup with an explicitly selected manifest captures the canonical allowed
root set, with `--root` among them. A later manifest cannot add a root without an explicit
restart. Native chat stays in the single selected root. Typed `forge-review-plan` and
`forge-review-run` operations share CSRF/origin checks, operation deduplication, revision
checks and the existing asynchronous control journal. The UI shows every parallel task,
PID, attempt, limit, root diff, review source/result/usage and gate diagnostics. Emergency
pause/cancel uses the pinned startup release even if a manifest edit makes inspection
unavailable. Publication/automation is not authorized by these operations.
