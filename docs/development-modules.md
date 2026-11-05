# Development modules

Harness and module are independent concepts. Codex is the initial harness. Feature
is the first delivery priority, followed by QA. Feature planning and scoped execution
are available. QA executes explicitly reviewed checks; refactor and CI/CD remain roadmap entries.

```
forge modules list
forge modules feature-plan /operator/proposal.json --root /projects/application
forge --state-dir /operator/state modules qa-plan /operator/releases/application.yaml --level standard
```

Feature planning accepts the existing structured proposal schema (identity, summary,
tasks with permanent IDs, dependencies and observable acceptance criteria with proposed
argv). It validates the DAG and builds context/estimates using the existing planner.
It does not execute commands or start native calls. Missing criteria are rejected.
Use the existing reviewed workflow to bind external evaluators before implementation.

QA planning uses a manifest with the existing release schema. Its repositories must exist and its
external evaluator contracts must validate.
State and evaluators must remain outside the target repositories, as required by
the existing execution contract. The result includes the existing
revision-bound native review plan and pending coverage. This command performs no
native calls, scanner invocations, corrections, deployments or state writes.

| Level | Planned coverage added |
| --- | --- |
| light | Quality/tests, secrets, dependency inventory/advisories |
| standard | Static security/ReDoS, auth/isolation, build supply chain |
| intensive | Bounded local adversarial tests and threat modeling |

Higher levels preserve lower-level checks. Every check starts as `not_run` with
unknown applicability and no evidence. Stack discovery and reviewed tools, budgets,
permissions and external evaluators are prerequisites to future execution.
`review run` alone does not execute this coverage plan. No level proves complete
security coverage or grants extra permissions.

Loop and checkpoint fields describe the planned contract, labeled `design_only`.
The QA planning fields do not enable automatic correction or context compaction. Future work must
persist checkpoints outside the LLM context and revalidate their revision and
artifacts before resuming. Module availability describes this interface, not the
availability of older workflow commands.

See [F064](../specs/064-development-modules/spec.md) for staged acceptance criteria.

## Execute a feature

Feature execution is available through the existing Codex workflow. Start with
[the small external-check example](../examples/feature-module.json) in a disposable
Git project. Copy/adapt the request outside the project and review its scripts.
The native Codex client must already be installed and authenticated; Forge preserves
its configured model/account. Python is required only by this example's check.

```sh
forge modules feature prepare /operator/request.json --root /projects/application --ops-root /operator/feature
forge modules feature prepare /operator/request.json --root /projects/application --ops-root /operator/feature --apply
forge modules feature start 001-greeting --root /projects/application --ops-root /operator/feature --approve-scope REVIEWED_DIGEST
forge modules feature status 001-greeting --root /projects/application --ops-root /operator/feature
forge modules feature checkpoint 001-greeting --root /projects/application --ops-root /operator/feature
forge modules feature resume 001-greeting --root /projects/application --ops-root /operator/feature
```

`prepare` previews by default; `--apply` persists the candidate, checks and pinned
Forge evaluator outside the project. It does not start the LLM. `start` binds consent
to the exact returned scope digest, materializes reviewed specs, and uses the existing
inspect/dry-run and execution path. Tasks run in dependency order with per-task external
checks. Each criterion ID, command argv and task scope must match an explicit check.
Missing or mismatched checks block preparation; a green unrelated check is insufficient.

Each task currently inherits the workflow's 900-second timeout and one attempt, plus
its existing context and budget policy. This is bounded incremental execution, not
an automatic retry-until-success loop. A failed check blocks completion and dependents.
Declared human gates remain pending until separately satisfied. No deployment, push,
release or service installation is authorized by this module.

`checkpoint` is a read-only context handoff built from the existing durable state.
It includes the objective, task DAG, source/evaluator/manifest fingerprints and limits;
current task evidence and usage remain in the accompanying report. Unknown usage stays
unknown. It neither writes an additional checkpoint file nor compacts the native context.
It cannot be imported as approval. `resume` reloads the authoritative journal, refuses
stale scope/evidence, and retains existing reconciliation requirements. Changed evaluators
block module inspection/execution until the operator restores or reviews a new contract.

## Execute QA

See [QA execution](qa-execution.md) for the strict request contract, immutable reviewed
scope, observed coverage, evidence, limits and recovery. Legacy `modules qa-plan`
remains a planning-only command with its existing manifest interface.

## Interactive terminal

Run `forge` in a Git project (or `forge --root PATH`). The first screen lets you choose:

- **Feature:** type the desired behavior. Enter creates a structured plan with checks.
  Review the plan, press Enter to prepare its authorization in the editor, then Enter
  again to execute that exact scope. Planning alone cannot start implementation.
- **QA:** enter a REQUEST JSON path to prepare without execution. Review exact argv,
  pinned inputs, source, coverage and gaps; Enter places `/qa-run DIGEST` in the editor,
  then another Enter executes. `/qa-status [ID]` reads evidence. `/qa LEVEL MANIFEST`
  retains legacy planning only. QA consent is separate from Feature consent.
- **Conversation — read only:** the existing native chat, without implicit workflow execution.
- **Refactor / CI/CD:** displayed as unavailable future modules.

`F3` or `/mode` (also `/modo`) reopens the selector, including from the plan/report panel. Changing mode clears the UI's selected
workflow/digest and is blocked during active work; persisted workflows remain available
through `/tasks`. Named slash commands retain their explicit meanings. Neither selection
nor a checkpoint grants native permissions or human acceptance. The browser Workbench
continues to expose its existing workflows; this mode picker is the terminal interface.
