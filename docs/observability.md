# Optional Loop-compatible observability

FORGE-EXP-OBS adds runtime spans without changing acceptance, execution results, client
permissions or publication authorization. SQLite and pinned evaluator evidence remain authoritative.
No provider or model fallback is implemented.

## Opt in

Existing manifests default to no telemetry. To enable the transport, the operator must set:

```yaml
automation:
  observability:
    langsmith: optional
```

The process must also have `LANGSMITH_TRACING=true` (exact lowercase value) and a nonempty
`LANGSMITH_API_KEY`. Credentials are read only from the environment, never serialized into
manifest/state/logs. `LANGSMITH_ENDPOINT` optionally selects an HTTPS endpoint; the default is
`https://api.smith.langchain.com`. `LANGSMITH_WORKSPACE_ID` optionally supplies `x-tenant-id`.
No project name, prompts, task descriptions, memory, paths, process output, environment dump,
exception messages, model names or token/cost payloads are sent. Inputs are always `{}`.
Inherited variables alone do not opt a manifest in. Inspect and dry-run initialize no transport.

Endpoint userinfo, query strings and fragments are refused. Redirects and proxies are disabled.
HTTP is accepted only by unit-test builds, only for literal loopback addresses. Production
binaries require HTTPS even on loopback. Invalid configuration degrades to noop.

## Payload and lifecycle

The adapter uses the official [basic REST tracing protocol](https://docs.langchain.com/langsmith/trace-with-api):
`POST /runs` at entry and `PATCH /runs/{id}` at exit, with UUIDs, UTC start/end timestamps,
`run_type: chain`, parent IDs and `x-api-key`. The service derives trace/dotted ordering from
parent IDs. The locally generated `execution_id` correlates the entire invocation independently
of provider-generated trace IDs. Each attempt gets a fresh span UUID; retries retain task ID and
increment attempt. Each invocation, including resume, gets a fresh execution ID.

Metadata is a typed allowlist: execution/release/task IDs, phase, attempt, dry_run, duration_ms,
and outcome. Release/task IDs are SHA-256 pseudonyms, preserving joins while suppressing raw
operator values (including secret-looking IDs). This is pseudonymization, not a promise against
dictionary attacks on predictable IDs. Values are at most 256 bytes; metadata is capped at 4 KiB.
Failure payloads contain only fixed status strings. Errors never contain task/provider details.

The worker queue holds at most 128 events (therefore at most 128 outstanding spans). Enqueue is
nonblocking; saturation discards events and increments a local counter. There are no retries.
Every request has a 2-second total/connect timeout; session drain has a separate total limit
of 2 seconds and aborts remaining I/O. Dropping a session aborts its worker. A cancelled future
emits a cancelled span when its guard is dropped; abrupt process termination can lose telemetry.
Partial traces under overload/failure are expected and never count as evidence.
Sanitized `dropped`/`failed` counters are best-effort SQLite metadata under `observability` for
opted-in invocations, without changing the original execution result.

## Loop mapping and limits of equivalence

The read-only [Loop inventory](loop-tracing-inventory.md) describes the inspected Python source.
The Rust mapping is:

| Span | Actual boundary |
| --- | --- |
| `forge.daemon_cycle` | One eligible release in a queue cycle, with that release's opt-in; disabled/paused/latched entries emit none |
| `forge.release` | Direct execution/resume, or the daemon's execution plus publication decision |
| `forge.task` | One executor attempt, including structured-client and read-only checks |
| `forge.validation_gates` | Each executed external acceptance gate |
| `forge.verification` | Acceptance evaluation, including artifact/human checks and final revision checks |
| `forge.publish` | An explicitly invoked publication operation; absent when publication is disabled or already recorded by daemon |

Task, gate and verification spans share the release parent, as in Loop. Verification includes
gate time in Forge because its evidence evaluation is one operation. Final re-evaluation emits
new gate/verification spans without inventing another task attempt. A publication span records
the operation's result, not proof of a remote publication; receipts remain authoritative.
Unlike Loop's global daemon sink, cycle traces are partitioned per eligible release so an opt-in
cannot leak neighboring release metadata. This is phase/contract equivalence, not byte-for-byte
SDK output, identical batching or identical duration boundaries.

## Evidence and pending acceptance

Deterministic local fake-server tests cover parent/run/attempt correlation, metadata bounds and
redaction, opt-in refusals, HTTP errors, redirects without credential forwarding, queue saturation,
timeout/drain, cancelled operations and worker teardown. CLI lifecycle tests check unchanged
failed-task results and dry-run behavior with telemetry configured, including production HTTP refusal.
No test requires real credentials, remote ingestion, an agent/model login or a paid API.

Real LangSmith ingestion/UI grouping, tenant authentication and deployment-specific endpoint behavior
remain pending separate credentials and authorization. Native platform, authenticated client and
publishing acceptance are not established by these tests. No publication, service activation,
commit, push, global configuration or durable memory write is part of this implementation.
