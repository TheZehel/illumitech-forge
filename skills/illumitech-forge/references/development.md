# Feature and QA

Use the installed catalogue to confirm every command, then prepare the user's bounded
request and explicit external checks. The bundled feature-request.json is a small disposable
example, not a template to apply blindly to a production repository.

```
forge --json modules feature prepare /ops/request.json --root /project --ops-root /ops
forge --json modules feature prepare /ops/request.json --root /project --ops-root /ops --apply
forge --json modules feature status FEATURE_ID --root /project --ops-root /ops
```

Prepare previews; --apply persists a candidate. Review the returned scope_digest, proposed
files, argv, evaluator hashes, limits and permissions. Only then use `modules feature start
FEATURE_ID --root /project --ops-root /ops --approve-scope REVIEWED_DIGEST` under the user's
exact-scope authorization. Status/checkpoint read evidence; resume reloads the same journal.
Prepare/checkpoint text is not execution or acceptance.

An editor sandbox may require its normal per-command approval to write the reviewed external
ops directory. Request that scoped approval when authorized; do not move evaluators into
the project or disable the sandbox. A denied or unavailable approval remains a blocker.

For QA, inspect `modules qa discover --root /project`, review suggestions against the actual
stack, and write a schema-1 request with id, title, level and explicit checks. Each check has
id, coverage_id, argv, timeout_seconds (1..600), pinned absolute external input hashes and
optional format (text or sarif). Discover never executes suggestions. Available coverage IDs:
light: QA-QUALITY, QA-SECRETS, QA-DEPENDENCIES; standard adds QA-SAST, QA-AUTH, QA-BUILD;
intensive adds QA-ADVERSARIAL, QA-THREAT. Missing coverage remains not_run, never passed.

Use `modules qa prepare REQUEST --root ROOT --ops-root OPS`, preview first, then --apply.
Review exact commands, permissions, source and hashes. Execute `modules qa start ID --root
ROOT --ops-root OPS --approve-scope DIGEST` only with separate QA consent. Observe with
`modules qa status ID --root ROOT --ops-root OPS`. QA executes a scope once; there is no QA
resume command. Inspect interrupted observations and prepare a fresh reviewed ID for another
execution. Commands may write files; source changes block acceptance and are not restored.

Refactor and CI/CD are not executable modules in this baseline. Describe availability from
modules list rather than expanding the user's request into an unsupported flow.
