# F068 — Current registry documentation

Tracks #15; delivery is coordinated in PR #14 alongside #13.

The published 1.7.0 crate embeds a separate README still headed 1.4.1. Users following
crates.io receive an obsolete installation command and cannot discover current first-run,
Feature/QA, continuity or portable-skill guidance.

- Describe the current product and installation accurately on crates.io, with working
  absolute documentation links and bounded platform/editor claims.
- Generate all four package READMEs from the same release version as current install docs.
- Reject stale source documentation before packaging and verify the actual README bytes
  inside each source archive before its extracted build.
- Publish as patch 1.7.1 alongside the already prepared F067 first-run fix. Published
  versions, tags and archive bytes remain immutable; publication needs current evidence
  and revision-bound owner approval.

No dependency, schema, telemetry, marketplace submission or website deployment is added.
