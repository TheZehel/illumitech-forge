# ADR 0002 — Foreground local workbench and structured native planning

Date: 2026-10-30. Status: implemented candidate; operator acceptance remains external.

The read-only dashboard remains unchanged. The workbench has its own loopback HTTP
transport and authorization boundary, rather than adding mutation to the dashboard.
React/TypeScript/Vite provides the UI; Rust owns project identity, JSON-RPC correlation,
external SQLite operation/session journals, sandbox scope and native process lifetime.
Production frontend assets are included and embedded from the CLI crate itself.

Compatibility is based on the operator-provided Codex CLI 0.166.0 generated JSON schemas
and the [official App Server documentation](https://learn.chatgpt.com/docs/app-server).
The connection sends `initialize` with client metadata and `experimentalApi:false`, waits
for the result, then sends `initialized`. Stdio JSONL is the supported transport. The
handshake's userAgent/platform fields are reported; no unobserved version or model access
is inferred. Versioned DTOs validate the workbench boundary; native event payloads retain
unknown fields for forward-compatible display. Unsupported native requests stay blocked.

Each workbench owns one exact canonical Git root, selected at startup. It creates native
threads in `read-only` sandbox with `on-request` approval; managed native policy can reject
those settings. Model/provider/effort, config layers and account are never overwritten.
Only native ChatGPT authentication permits turns; API-key fallback is prohibited. Missing
login directs the operator to the native client rather than accepting tokens in the UI.
No account email, authentication payload, config secret or credential file is exposed.

A random local capability in the startup URL fragment bootstraps a strict HttpOnly cookie;
all project/control reads require that session and an explicit CSRF header. Controls also
require exact Origin. Host and Fetch Metadata checks prevent DNS rebinding and cross-site
access. No wildcard CORS, remote bind, file-serving endpoint or UI executable override
exists. The operator-only executable flag is passed directly to process spawn without a
shell. Native command/file permission requests require a matching pending callback and a
user-selected native decision. Such decisions do not accept a Forge revision.

Operation IDs are durably reserved before native submission. Repetition, timeout,
disconnection and restart never automatically replay a turn or approval. Reconnect replaces
the transport only; threads need explicit resume. Unknown outcomes remain unknown. Display
buffers are bounded; a transport gap blocks the connection rather than pretending delivery.

Assisted planning uses a separate ephemeral read-only thread with a versioned outputSchema,
a total deadline, bounded transport, explicit errors and no auto-approved requests. Only a
validated DAG is saved. Proposed commands are data; manifests retain read-only executors,
optional memory, disabled automation and human evidence gates until the operator pins
external evaluators. The same duration estimator and bounded-context contract used by the
engine provide projections. Missing measurements remain null; forecasts are uncertain.

Desktop packaging, remote access, full native request-form support, paid API fallback,
implicit session import and execution/publication from chat are outside this decision.
