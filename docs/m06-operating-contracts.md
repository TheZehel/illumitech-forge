# M06 local product contracts

Implements FORGE-026–029 on the existing engine and workbench. These interfaces do not grant
publication, CI, account/model changes or native platform acceptance. Milestone acceptance
belongs to the coordinator's external ledger, evaluators and real client/UI checks.

## Versioned templates

```sh
illumitech-forge --json templates list
illumitech-forge --json templates plan rust-feature --root PROJECT --id 001-example --ops-root OPERATIONS
illumitech-forge --json templates apply rust-feature --root PROJECT --id 001-example --ops-root OPERATIONS --apply
illumitech-forge --json templates update rust-feature --root PROJECT --id 001-example --ops-root OPERATIONS
```

`PROJECT` must be an existing Git project. `OPERATIONS` must be outside Git projects.
`plan`, `apply` without `--apply`, and `update` without `--apply` only render previews.
Optional `--title` supplies intent; otherwise the permanent spec ID is used as the title.
The versioned `schema_version: 1` inventory contains `rust`, `node` and `generic` recipes for
`bug`, `feature`, `review`, `dependencies` and `release` (15 combinations). Each recipe has
`id`, `version`, `source`/`origin`, content-derived `revision`, `scope`, `compatibility`,
`profiles` and `executable: null`, using the catalog Recipe format. No URL fetch or recipe
command execution exists. Commands in the recipe are suggestions for operator review.

Rendering delegates to guided creation: spec/plan/tasks plus an external, read-only, human-gated
manifest. Stack detection and existing conventions remain authoritative. Memory is optional,
CI/actions and automation are off. Files are create-only, symlink/traversal checked and
preflighted together. Repeating an identical application is a no-op. Provenance is stored in
`.forge-toolkit/templates/SPEC_ID.json` with generated file digests.

Every preview provides old/new content, hashes and an action for each path. A differing local
file is never overwritten. `update --apply` produces a content-addressed merge candidate under
`.forge-toolkit/template-updates/` when content or provenance differs. It preserves all current
files, including local customizations; an operator merges this candidate explicitly. Updating
is deliberately conservative, rather than an automatic three-way merge. Unknown/remote IDs fail.

## Budget contract

Manifests and individual tasks optionally accept this strict, additive object:

```yaml
budget:
  max_tokens: 100000
  max_active_seconds: 900
  max_attempts: 3
  max_concurrency: 1
  actions: off
```

All numeric ceilings are optional. Zero is an exhausted ceiling. Missing ceilings preserve
legacy scheduling; they do not describe a free account balance. The default scheduler remains serial. M07 accepts concurrency 1..32 and bounds opt-in
parallel worktree waves with `execution.max_parallel`; see [M07](m07-operating-contracts.md). `max_cost_micros` is
recognized, but blocks native inference because native subscription cost is not observable.
No native rate, monetary savings or account balance is inferred from tokens. Cached input is
already included in input; total is input + output.

`budget inspect MANIFEST` loads the current contract read-only and emits `policy`, task
policies, `observed`, `available_balance: null`, `cost_micros: null`, `actions: "off"` and
limitations. It never creates operational state or calls a client. Unknown counters are null.
When state exists, attempt and usage totals cover the complete journal, including reconciled
runs. The `report` and workbench expose the same policy and counters.

Admission is checked before every task invocation, including native retries. Token budget zero
blocks before spawn. After an unmeasured/partial native call, further token-budgeted inference
fails closed. Observed failed calls still consume budget. A call may exceed the remaining
token budget internally: the configured native client provides no hard per-inference ceiling.
The product never changes model, reasoning, account, provider or prompt policy to meet a budget.

Task attempts remain bounded by `max_attempts`; release/task budget attempts additionally count
the durable invocation journal and cannot be reset by reconciliation. Commands and tests consume
time and attempts, but do not invent native token/currency counters. Timeouts are capped by the
remaining active-time policy, conservatively rounded to whole seconds. Active time includes
execution, gates and engine bookkeeping. Forge human review wait and paused time are excluded.
Unknown legacy/interrupted time blocks time-budgeted admission. Limits remain cumulative across
reconciliation. Human approval remains valid only for its evaluated revision.

Workbench planning and chat attached via `--manifest` share that release's token/attempt/time
policy. Standalone planning can attach it with `plan propose ... --manifest MANIFEST`; dry-run
still makes no calls or state writes. Non-attached planning has no release budget, and reports
its own measured usage. Attached calls reserve a durable unknown record before native startup,
record measured usage even when proposal validation fails, and retain unknown usage on error,
timeout or disconnection. A worktree lease serializes them against engine execution. Native
planning has a bounded deadline; budgeted chat shuts down its owned native connection at the
time deadline without replay. Native tool approval wait counts toward that native call deadline.
CI activation is unavailable in this milestone: `actions: on` fails validation. No workflow runs.

## Validation reuse

A gate opts in explicitly; existing gates always run normally, including final evaluation:

```yaml
cache:
  enabled: true
  ttl_seconds: 300
  environment: {} # optional exact inherited environment assertions
  toolchain: {}   # optional absolute local toolchain/configuration path -> SHA-256
```

Enabling cache declares a deterministic local evaluator with all relevant external dependencies
pinned. Do not enable it for network access, changing external services, hidden dependencies or
side-effecting operations. Native clients, remote tooling and shell commands are rejected from
reuse. The evaluator still runs independently of agent prose; its pinned inputs must remain
outside all task worktrees. Human gates and artifact checks are never cache entries.

A key covers the source revision and canonical worktree, staged/untracked content, ignored files,
file modes, all declared roots, full manifest hash/policy, task/dependencies, gate argv/input
hashes, runtime and evaluator file modes/content, the Forge binary digest, declared toolchain hashes,
OS/kernel identity and a
hash of the complete inherited environment. Environment values are never stored in cache
provenance. Traversal is bounded to 10,000 files and 64 MiB; symlinks, unobservable files,
missing executables/declarations or mismatched declared environment/toolchain cause a miss.
Large build trees therefore often disable reuse conservatively. Include every relevant ignored
artifact/configuration within these bounds or leave reuse disabled.

Only a passing gate that leaves the complete key unchanged is saved. Entries have a checksum,
creation time, evidence digest and key. TTL must be 1–86400 seconds; expired/future timestamps,
missing evidence, invalid checksums, malformed entries, policy changes or changed inputs cause
fresh evaluation. Each release/task/gate keeps one entry, replacing the previous candidate;
TTL controls reuse, and stale entries are overwritten on the next passing evaluation.
State is stored in the protected operational SQLite database, never the agent worktree.
The cache is an optimization, not a sandbox: the operator remains responsible for accurately
declaring deterministic evaluator inputs. Hidden network state cannot be proven equivalent.

`gate_cache` journal events contain task/gate, hit/miss, reason, key and TTL. Evidence on a hit
retains the original gate digest and provenance. Cache hits still pass through engine evidence,
journal, timing and final-revision checks. Human review is never reused for a changed revision.
No token savings are claimed without measurements.

## Local controls and human review

The legacy dashboard remains read-only. Only the authenticated workbench exposes writes.
Startup still requires an explicitly supplied project and optional external manifest. `GET
/api/forge` returns `control_revision` (opaque manifest + current source hash), inspection,
`operation_active`, operational state/report, budget, review diff/fingerprints and the latest
control journal. The UI keeps review controls disabled during an asynchronous run; pause and
cancel remain available.
Untracked file contents contribute to the fingerprint; review their contents locally as well
as the displayed tracked-file diff before approving.

POST controls accept JSON with a permanent `operation_id`:

| Route | Additional fields | Behavior |
| --- | --- | --- |
| `/api/forge-start` | `revision` | Start the supplied manifest asynchronously |
| `/api/forge-pause` | none | Cooperative pause at the next task/gate boundary |
| `/api/forge-cancel` | none | Cancel the current owned process group |
| `/api/forge-resume` | `revision`, optional `reconcile` reason | Resume through engine reconciliation rules |
| `/api/forge-approve` | `revision`, `task`, `gate`, `reviewer`, `fingerprint` | Engine revision-bound human approval |
| `/api/forge-deny` | same review identity fields | Revoke this gate's prior approval; never resume automatically |

Start/resume/approval/denial require a connected authenticated native account and a fresh
revision token. Approvals must match the current evaluated task fingerprint and human gate.
The UI requires checking the current diff and naming the reviewer. Source changes clear that
review selection. A native tool approval never grants Forge acceptance. Changed source or
manifest cannot transfer approval; resume must reconcile explicitly where the engine requires it.
An altered project/release scope fails closed. UI execution rejects automation/publication.

Pause/cancel use the startup-pinned project/release identity, without requiring unchanged source
or reparsing a manifest that an authorized writer may currently be changing. The existing
engine controls remain authoritative. Shutdown requests cancellation and waits for owned work.

Every request still requires exact loopback Host, local Origin for writes, authenticated
HttpOnly/SameSite cookie and capability-bound CSRF header. Capability entropy comes from the OS
through locked `getrandom`, with no timestamp fallback. Unknown routes/fields and unrelated
control fields are rejected. An operation ID is reserved durably before execution and is never
replayed, even after process restart or an uncertain outcome. Request journal entries record
reserved/accepted/rejected/finished states; the engine journal records actual operational
outcomes. Asynchronous acceptance does not mean a run passed. A lost connection grants nothing.

## Security and compatibility follow-through

Known credential context paths (`.env*`, `.ssh`, `.aws`, native auth files, private key files,
credentials and related directories) are excluded from reads; explicit sources are rejected.
JSON named credential values are redacted wholly, including objects/arrays, while protocol
IDs/types and token counters retain their structure. App Server shutdown, disconnect and drop
request native turn interruption before terminating owned process groups, even if the parent
exits before descendants. Detached native tool groups still require real-client acceptance;
local process fixtures cannot establish their lifecycle behavior. These boundaries
have local regressions, not cross-platform native acceptance.

UI assets are generated by the locked build, embedded from the CLI package's own `ui-dist/`,
and include React/React DOM/scheduler MIT notices. The CLI package can serve them without an
ancestor source checkout. Domain must be packaged before adapters, which now depends on domain.
Windows runtime and Claude operation remain deferred. No installation, hook, service, remote
publication or account change is part of these contracts.
