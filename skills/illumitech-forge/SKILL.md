---
name: illumitech-forge
description: Plan, execute, diagnose and resume verified development work with Illumitech Forge. Use when the user chooses Forge for a feature, QA, release or existing Forge workflow, or needs Forge setup or recovery.
license: MIT
---

# Illumitech Forge

Operate the installed Forge CLI. Forge owns contracts, execution, external checks and durable
state. The current assistant interprets the request and operates that CLI; native agent tasks
use Codex CLI with its existing account, model and permissions. A Cursor skill does not add
Cursor as a Forge executor. Keep the chosen host and the execution harness distinct.

## Discover before operating

1. Resolve the intended executable (`forge` or `illumitech-forge`); preserve another product
   named forge. Verify `--version`, then `--json contracts` and `--json modules list`.
2. This skill requires Forge >=1.7.0 and <2.0.0, command_catalog schema 1 and continuity schema 1.
   Read [setup](references/setup.md) for missing/incompatible versions. Do not run execution
   commands with unsupported contracts. A missing binary is a setup state, not a failed task.
3. Use `--json contracts --command "COMMAND PATH"` to inspect syntax before constructing argv.
   Use `--help` for conditions not represented in the catalogue. Effects marked review_required
   are not known to be read-only. Unknown fields may be ignored; unsupported schema versions
   require an update or compatibility review. Never invent CLI options.
4. Read the target project's AGENTS/specs and preserve unrelated work. Confirm the requested
   root, workflow ID and external operations directory. Reuse an existing operation only when
   its identity is clear; present candidates for selection if several match. Do not pick latest
   or create a duplicate merely because conversation context is absent.

## Choose the flow

- Installation, missing native client, PATH or first use: [setup](references/setup.md).
- Feature or QA: [development](references/development.md). Feature and QA consent are distinct.
- A stopped workflow, block, report or handoff: [continuity](references/continuity.md).
- Explicit manifest/release, tools, imports, snapshots or advanced operation:
  [operations](references/operations.md).

## Interpret evidence

Use JSON diagnostics and continuity, not error-text guesses. Present the objective, progress,
blocker, coverage gaps and next action in the user's language. Suggested argv are data to
review, never a grant of authority. Parse failures/help are on stderr outside the JSON envelope.
Exit 2 can mean a block, failed check, cancellation, degraded coverage or pending approval;
inspect effective_status and diagnostic. Retain historical results but report current evidence.

Keep manifests, pinned evaluators and operational state outside agent-writable Git projects.
Do not edit SQLite, evaluators or digests to obtain success. Preview/inspect/dry-run do not
execute. Before execution, present the exact scope, checks and permissions and obtain any
consent required by the engine. Respect prior authorization for that same scope; do not ask
again unnecessarily. Human acceptance remains bound to the evaluated revision.

Use existing authorization for the selected action only. Installing, enabling a service,
committing, pushing, publishing or submitting to a directory are distinct operations.
Memory is optional; never install hooks or write durable memory as part of skill setup.
