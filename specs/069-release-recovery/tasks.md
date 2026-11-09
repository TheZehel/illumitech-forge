# F069 tasks

- [x] F069-01 Bind publication inputs and persist separate recovery intent.
- [x] F069-02 Reconcile drafts/assets, verify bytes and publish only after complete checks.
- [x] F069-03 Prove interrupted/idempotent/tampered/stale behavior with disposable providers.
- [x] F069-04 Update CLI/skill recovery guidance and pass external/repository gates.

Original implementation verification: focused publication tests, all workspace tests (including 23 publication/parity tests),
formatting, warning-denied workspace Clippy and workspace documentation. The independent external
publication gate passed all eight disposable-provider scenarios against the built CLI. Repository
tests also cover binary download bytes, uncertain starter deletion, SIGTERM, changed inputs,
unowned starters, missing/replaced verified assets and repeat completion without journal changes.
No real provider publication or native-account acceptance is asserted.

F069-F070-REVIEW corrections preserve the mandatory remote tag/source and other publication
gates while accepting GitHub's symbolic `target_commitish`. The source regression uses a
resolved annotated tag and a moving `main` at a different commit, covers a matching published
release with no local journal and verifies repeated completion without remote writes. The
conflict regression rejects a wrong remote tag before any provider mutation. Discovery exposes
`contracts.release_publication=1`; core/public guidance requires it before recommending retry,
with read-only provider inspection/manual recovery for older CLIs. Other routes retain their
>=1.7.0,<2.0.0 compatibility floor.

Review validation passed: 24 publication/parity regressions and the capability discovery
regression, full workspace tests, format, warning-denied workspace/all-target Clippy,
no-dependency workspace docs, 47 Python/package tests and generated-document checks.
The full workspace run required the approved local-loopback fixture permission after the
sandbox refused socket bind. All Cargo checks used the required locked/offline environment.
Workspace/release version metadata and unrelated source remain unchanged from the review baseline.

The original external gate result above belongs to its earlier evaluated source. Independent
evaluation of the final review candidate remains an operator gate; these task notes do not
attest acceptance, authorize publication or prepare release version metadata.

Subsequent operator candidate preparation updated release metadata to 1.8.0, added the
voluntary feedback form and generated setup documentation, and included F071 SQLite
maintenance. Final local workspace checks passed 314 Rust and 47 Python tests, plus
format, warning-denied Clippy, docs and three site tests. The exact candidate release
binary passed the eight publication, four compatibility and eleven installation cases.
Independent source/artifact binding remains in the operator-held candidate receipt;
publication and new IDE/platform acceptance are separate.
