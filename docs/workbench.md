# Workbench, guided creation and planning — candidate 1.0

Implementation documentation; milestone acceptance is owned by external evaluators.
The existing dashboard, manifest v1, FORGE-020 duration and M04 execution/economy APIs remain
available. Chat statements are never gate evidence.

## Build and workbench

```sh
npm ci --prefix apps/forge-ui --ignore-scripts
npm run typecheck --prefix apps/forge-ui
npm run build --prefix apps/forge-ui
cargo build -p illumitech-forge-cli
illumitech-forge --json --state-dir EXTERNAL_STATE workbench --root ABS_GIT_ROOT --port 0
```

The first startup JSON line is `{"event":"workbench.started","url":"...","pid":123}`.
The URL is a local capability; open it yourself and keep it private. The server binds only
127.0.0.1 and serves packaged UI/assets at the same origin. Ctrl+C stops the foreground
server and native child. An unavailable native executable leaves an honest disconnected UI.
`--app-server PATH` is an operator-only native executable override, never a browser field.
Its arguments remain `app-server --listen stdio://`. No shell is involved.

Only workbench-created sessions are listed; SQLite records project/session ownership and
submitted operation IDs outside Git. Message buffers stay in bounded memory; resume requests
native history explicitly. Opening/reloading the UI does not replay writes. Reconnect does
not resume threads or repeat unknown turns. A duplicate operation ID fails, including after
restart. When an outcome is unknown, inspect/resume before deciding on a new action.

The sidebar selects the authorized project/session. The conversation streams native agent,
command, file-change, diff and plan events. Native decisions include refusal/cancellation
and available grants; unsupported native request types display blocked. Native tokens are
observed counters, with cache included in input. No usage event means unavailable, not zero.
Tabs support arrows/Home/End, controls have visible focus, and Ctrl/Cmd+Enter sends.

Interruption requests native `turn/interrupt` and requires a typed interrupted outcome.
It does not promise to terminate every native background terminal. In the real 0.166.0
pilot a Python descendant remained alive after the interrupted turn; inspect such terminals
with the native client. Forge release cancellation separately stops its owned workers.
Explicit resume reconstructs completed/failed/interrupted history from native turn metadata;
unknown status is never displayed as completed, and resuming starts no inference.

`--manifest EXTERNAL_MANIFEST` enables engine inspect/status/report projections, including
DAG, evidence, usage and duration, only for canonical roots explicitly captured at startup, with the selected root among them.
No arbitrary manifest/file path is accepted from the browser. Without the flag the UI states
that evidence is unavailable. Local typed execution/pause/cancel/resume/revision approval controls are described in M06.
M07 adds multi-root projections and independent review. Publication contracts cannot be
started through the UI; there is no publish endpoint.

## Guided creation

```sh
illumitech-forge --json feature new --root ABS_GIT_ROOT --id 001-greeting \
  --title "Greeting feature" --ops-root EXTERNAL_OPS --dry-run
illumitech-forge --json feature new --root ABS_GIT_ROOT --id 001-greeting \
  --title "Greeting feature" --ops-root EXTERNAL_OPS --apply
```

Preview is the default and performs no writes, state creation, installer or client call.
Apply creates `specs/001-greeting/{spec,plan,tasks}.md` and
`EXTERNAL_OPS/releases/001-greeting.yaml`. IDs and dependencies are deterministic. Existing
identical files are retained; every conflict is checked before writes and reported without
overwriting the operator's content. Interrupted apply can be repeated. Unsafe IDs, `..`,
symlinked paths and operations inside Git are rejected. Stack detection only checks existing
convention files; no dependency install is implied. Generic titles cannot define exact
product behavior, so unknown acceptance/evaluator details remain explicit review requirements.

The UI offers the same preview/apply flow. Its candidates use a dedicated directory under
the operator-selected external state root, never a browser-selected filesystem root.

## Planning schema v1

```sh
illumitech-forge --json plan propose --root ABS_GIT_ROOT --request "Add a greeting" \
  --ops-root EXTERNAL_OPS --output EXTERNAL_CANDIDATE --dry-run
illumitech-forge --json plan propose --root ABS_GIT_ROOT --request "Add a greeting" \
  --ops-root EXTERNAL_OPS --output EXTERNAL_CANDIDATE
illumitech-forge --json plan validate EXTERNAL_CANDIDATE --root ABS_GIT_ROOT
```

Dry-run has no model call or writes. Without it, the configured native ChatGPT client
produces structured JSON in an ephemeral read-only thread. No model, account or config is
selected by Forge. Approval requests, invalid JSON, failed/interrupted turns, disconnection,
unknown dependencies, cycles, duplicate IDs and missing acceptance prevent candidate saving.
A 120-second proposal deadline interrupts the turn; RPC/frame limits also apply. There are
no speculative inference retries. `plan validate` accepts JSON/YAML, is deterministic and
never invokes a client. Output is create-only and external. Native proposals never execute.

The versioned schema is `crates/forge-cli/src/planning-v1.schema.json`:

```json
{
  "schema_version": 1,
  "id": "001-greeting",
  "title": "Greeting",
  "summary": "Return an observable greeting",
  "tasks": [{
    "id": "greeting-T01",
    "title": "Implement greeting",
    "dependencies": [],
    "acceptance": [{
      "id": "greeting-AC1",
      "description": "External regression verifies the exact greeting and failure behavior",
      "command": ["cargo", "test"]
    }]
  }]
}
```

All proposal/task/criterion fields are required; unknown fields fail validation. Acceptance
commands are bounded argv arrays for review, never executed or trusted as evaluator authority.
The saved bundle contains `proposal`, safe manifest v1, proposed spec/plan/tasks documents,
DAG order, duration forecast, bounded-context preflight, request digest and native usage when
reported. The external manifest has only read-only native executors and revision-bound human
gates; reviewers must pin external evaluators and authorize any execution permissions.
Native usage absence is null. Duration has a range and assumptions, no promised ETA or zero
cost fabrication. Missing candidate specs on disk are shown as blocked context until reviewed
and prepared; the candidate embeds proposed documents without overwriting the repository.

`--previous PREVIOUS_CANDIDATE` validates a prior proposal and records added/removed/changed
permanent task IDs, previous/current duration and a digest. The new proposal's context is
recomputed. Scope changes never inherit evidence or execution/publication authority.

## HTTP contract

`GET /api/health` is minimal liveness. `POST /api/connect` requires exact Origin and the
startup fragment capability in `X-Forge-Token`; it sets an HttpOnly SameSite=Strict cookie.
Other APIs require that cookie plus `X-Forge-CSRF`. POST controls additionally require Origin.
Host must be exactly the issued 127.0.0.1 host/port; cross-site Fetch Metadata is rejected.
There is no CORS allow-origin. Responses are no-store with a same-origin CSP and no framing.

`GET /api/state` provides a schema-v1 snapshot and `GET /api/forge` reads the opt-in manifest.
POST routes are `/api/sessions`, `/api/resume`, `/api/turn`, `/api/interrupt`, `/api/approval`,
`/api/reconnect`, `/api/feature-preview`, `/api/feature-apply`, `/api/planning-preview` and
`/api/planning-propose`. Every control requires a unique `operation_id`; session controls
also require an owned `thread_id`. Turns have `text`; approval responses carry the original
`request_id` and one exact `decision` from available native choices. Feature controls accept
`id`/`title`; planning accepts `request`. No credential, model/config/permission override,
arbitrary path or executable can be supplied by a browser. Errors never auto-approve or replay.

The UI retains the local capability in tab-scoped sessionStorage for reload; it never contains
native account credentials. Treat same-origin browser extensions and local operators as trusted.

M06 adds project-scoped execution, pause/cancel/resume and explicit revision review in the
Evidence panel. Its [public control and budget contracts](m06-operating-contracts.md) describe
Origin/CSRF, idempotency, async outcomes, current fingerprints and safe cancellation.

M07 extends explicitly selected manifests to a pinned set of non-overlapping roots and
adds independent native review, per-worker states, global controls and verified gate
diagnostics. HTTP routes and session ownership are scoped to the selected project. Native read-only sandbox semantics can permit reads outside that root and inherit configured MCP tools; this is not an OS read-confinement guarantee. See the
[M07 operating contracts](m07-operating-contracts.md).

## Scope and current evidence

See [candidate 1.0](v1-candidate.md) for versioning, budgets, recovery and platform limits.
HTTP/session ownership is project scoped. Native filesystem and MCP read visibility follows
the inherited client configuration; it is not a universal repo-only OS read sandbox.
Session labels persist as short generic names, without copying full prompts into ownership
tables. Final-source native acceptance belongs to the operator ledger.
