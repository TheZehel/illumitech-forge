# F067 plan

Probe git rev-parse --verify HEAD^{commit} only for the selected valid worktree, with the
existing bounded process runner. Expose presence, absence and unavailable observations as
available, missing and unverified. Include git_revision in Feature prerequisites and update
setup guidance. Do not run a revision probe against an invalid project's fallback directory.

Use a disposable repository to prove the transition from unborn to committed, preserve a
sentinel file, and verify that doctor neither runs inference nor creates execution state.
Run repository formatting, tests, Clippy and docs. Carry publication separately; this patch
does not rewrite the approved 1.7.0 release.
