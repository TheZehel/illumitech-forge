# F067 — Diagnose an unborn Git repository

The editor pilot found that workflow doctor accepted a Git worktree without HEAD, then
Feature preparation failed while reading its revision. Explain that missing prerequisite
before preparation and keep the skill's first-run guidance consistent.

Tracks #13. Editor pilot continuity is tracked separately in #12.

- Doctor reports a separate git_revision check. An unborn worktree is blocked with an
  actionable explanation that an initial commit needs operator review.
- A worktree with a commit can pass this check. Probe failures stay unverified.
- The check is bounded and read-only. It creates no commit, file, state or inference.
- Project, client/authentication and PATH checks retain their existing meaning. Skills
  never fix installation conflicts or create initial commits implicitly.
- The already published v1.7.0 tag and its artifacts remain immutable.
