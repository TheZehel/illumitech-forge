# ADR 0001 — Evidence and authority

Accepted design, implementation acceptance tracked separately.

Specs/ADRs own intent; YAML owns execution; SQLite owns state; Git owns code;
ai-memory owns continuity. Executors cannot approve their own work with text.
Each run binds manifest bytes, canonical repo roots, Git revision, full tracked/untracked
worktree fingerprint and effective executor configuration. Gates are commands, never shell
strings. Human approval references the exact evaluated fingerprint. Imported Loop state
never confers acceptance. Publication is a separate explicit operation.

State and evaluator code must be outside repositories writable by the executor. Local filesystem
owners can still tamper with state: this is a trusted local orchestrator, not an adversarial OS sandbox.
Read-only inspections do not create SQLite or locks. A crash releases OS locks but a running
ledger entry must be reconciled; never automatically rerun an orphan.
