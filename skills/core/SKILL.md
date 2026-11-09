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

Before advising recovery of interrupted release publication, check the selected executable's
`--json contracts` for `contracts.release_publication=1`. Without that capability (including
published 1.7.0/1.7.1), preserve state, inspect provider state read-only and arrange manual operator
recovery; do not repeat the nonrecoverable publisher or create another release. Other skill routes
still support Forge >=1.7.0,<2.0.0 with their required contracts.

Only with that capability, preserve external state and retry the same explicitly authorized
`release publish` inputs. Reconcile provider identity, inventory and downloaded bytes; never rerun
implementation or transfer stale human approval. Release recovery uses its own journal, separate
from draft PRs. See `docs/release-recovery.md`; remote conflicts require operator review.
