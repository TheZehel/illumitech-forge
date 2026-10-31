# Native protocol reliability (milestone 0.4)

FORGE-021 preserves JSON/JSONL framing while redacting sensitive string values. Nested objects,
arrays and escaped/multiline strings are sanitized without replacing a structured event with
an unparseable line. Event types, IDs, numeric usage, error codes and terminal events remain
available to protocol validation. Plain text is still redacted line by line. No credentials
or raw environment values are required by the regression fixtures.

Exit 0 alone never completes a native task. Codex requires valid JSONL, no failure/error event
and `turn.completed`; Claude requires the successful JSON result and no permission denials.
A sanitized reproduction of the FORGE-020 failure shape covers exit 0, sensitive item output
and missing terminal completion. It remains blocked after redaction. Local fixtures test the
shape; the coordinator's fixed external fixture and real native job remain independent gates.

## Retry decisions

| Failure class | Automatic behavior |
| --- | --- |
| Explicit native transient error (rate limit, server unavailability, connection interruption) | Retry within the declared max_attempts, never more than three total; 250/500 ms bounded backoff. |
| Permission or authentication denial | Block, journal the reason, require inspection/reconciliation; do not change account, model, tools or permissions. |
| Missing/malformed native protocol, unknown native failure, spawn failure | Block; no blind retry or text-based success. |
| Timeout, output limit or cancellation | Stop without automatic native replay. Keep partial usage and precise process reason. |
| Failed artifact/evaluator evidence after native execution | Block for explicit intervention; do not automatically launch the agent again to repeat the same work. |
| Declared command executor failure or failed gate | Preserve its existing bounded retry contract and FORGE-020 per-attempt timing; recognized permission/auth and blocked evidence stop. |

Error classification inspects structured error payloads and stderr, not agent prose or usage
counter values. Permission/auth/contract failures take precedence over transients. The task's
execution evidence includes failure class and retryability; per-call usage has its own outcome.
Passing execution is still followed by all external artifact/gate/human checks, including final
revision verification. This increment transfers no approval across a revision or reconciliation.

## Journals, recovery and bounded reads

The SQLite event journal remains append-only. `Store::events` remains a complete recovery read;
`recent_events` and `events_page` are separate bounded projections. CLI report accepts
`--limit 1..1000` and `--offset`, with newest events/calls first. Totals include the complete
usage journal regardless of the requested page. See [token-economy.md](token-economy.md).

Logs now use `logs/RELEASE/generation-N/TASK-ATTEMPT.json`. Every attempt is retained, and
reconciliation creates a new generation rather than overwriting old captures. Existing legacy
log files are left in place. Captures are redacted and limited to 1 MiB per stream. Overflow
terminates execution and marks it incomplete; it is never silent protocol success. Operator
retention/archiving is explicit and separate from recovery or report pagination.

This implementation does not change services, hooks, native accounts/configuration, evaluator
files, orchestration manifests, publication or client permissions. Native validation and
milestone acceptance remain external to the repository and to agent prose.
