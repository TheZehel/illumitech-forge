# Illumitech Forge 1.6.0

Compatible feature release based on 1.5.0.

- Interactive terminal selector for Feature, QA and read-only Conversation. F3 or
  `/mode` switches modes; unavailable Refactor and CI/CD remain visible.
- Feature preparation, explicit scope approval, per-task external checks, status,
  resume and a read-only context handoff through the existing workflow engine.
- QA discovery, immutable preparation, explicit execution and evidence inspection.
  Light, standard and intensive levels report observed coverage and missing checks.
- Bounded subprocesses, SARIF findings, cancellation, source/evaluator revalidation
  and replay prevention. Incomplete coverage never produces a passing QA result.

QA requires reviewed commands and pinned external inputs; it does not install scanners,
provide an OS read-only sandbox or attest universal security. Automatic correction,
context compaction, Refactor and CI/CD remain future work. The selector is terminal-only;
existing browser workflows are preserved.

Native release scope: macOS ARM64. Linux native acceptance remains deferred (#6);
Windows and other native harnesses remain experimental. No site deployment is included.
See the release verification asset for checksums and final validation evidence.

Usage: run `forge` in a Git repository, select a mode, and review the scope before
confirming execution. See [development modules](development-modules.md) and
[QA execution](qa-execution.md) for strict request examples and operational limits.
