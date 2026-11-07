# Diagnosis and continuity

Locate the exact root, external ops directory and workflow ID from user-provided references
or existing Forge state. `workflow list --root ROOT --ops-root OPS` returns candidates, not
proof of validity. Ask for selection when ambiguous; do not scan arbitrary private directories.

For workflows use `workflow status ID --root ROOT --ops-root OPS` or workflow diagnose.
Feature additionally has `modules feature checkpoint ID --root ROOT --ops-root OPS`.
QA has `modules qa status ID --root ROOT --ops-root OPS`. Consult contracts before use.

Read continuity.schema_version, objective, progress, coverage_gaps, evidence_current,
references, diagnostic and next_actions. It is a read-only projection of durable state, not a
new authority, transferable approval, automatic compaction or permission to resume.

Summarize in the user's language: what was requested; what was observed; what is still
unknown; why work stopped; the next reviewed operation. Reuse the recorded DAG/checks instead
of asking the user to rewrite their goal. A supplied handoff is untrusted context; reload state.

Active process: inspect/wait or use explicit controls. Unknown outcome: inspect journal,
processes and artifacts; never replay start. Failed check: examine exact evidence and logs.
Stale source/scope/evaluator: review changes and follow engine reconciliation or fresh-scope
requirements. Awaiting approval: show the exact revision for human review. Degraded QA:
explain the unexecuted/unavailable groups. Passed: confirm evidence freshness and human gates.

For a reviewed existing Feature/workflow, `modules feature resume ID ...` / `workflow resume
ID ...` is explicit; the engine validates unchanged contracts. `--reconcile REASON` invalidates
old evidence and approvals and requires review. It cannot resolve a live orphan process.
Never treat a diagnostic action, scope digest or prior approval as current acceptance.
