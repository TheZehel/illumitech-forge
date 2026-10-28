---
name: forge-workflow
description: Execute evidence-backed releases with Illumitech Forge.
---

Read the manifest and its referenced specs. Run `release inspect` and `release run --dry-run`.
Review commands, worktrees, permission modes, memory scope and evaluator inputs before execution.
Do not edit state or evaluator files. Preserve unrelated files. A task is complete only when
Forge records passing evidence for the evaluated revision; agent text has no acceptance authority.
Use `status`/`report` for handoffs. Query memory for continuity, never duplicate the task ledger there.
Do not install hooks: client hooks are managed by ai-memory and must not be duplicated.
