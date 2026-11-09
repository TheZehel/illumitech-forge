# F070 tasks

- [x] F070-01 Implement bounded preview/apply project installer and package it (local source).
- [x] F070-02 Prove compatibility, checksums, unsafe ZIP rejection and preservation (local regressions).
- [x] F070-03 Align skill setup, public tutorial and voluntary-feedback instructions.
- [x] F070-04 Validate pinned public catalog installation and current first-use evidence (public v1.7.1).

Local evidence: `python3 -m unittest discover -s scripts -p 'test_*.py'` passed 47 tests,
including real standalone ZIP bytes, both project layouts, default read-only preview,
explicit apply, compatible/old/missing/schema-incompatible CLI, outer/inner hashes and CRC,
wrong/plugin/extra/missing/traversal/duplicate/symlink/special-file ZIPs, expansion/probe
bounds, existing target/unrelated-file preservation, symlink ancestors, exclusive creation
and rollback. The binary-package regression executes the shipped installer independently
with Python isolated mode. A separate disposable check used the locally compiled 1.7.1 CLI
and installer extracted from a real binary archive for preview/apply/repeat preservation in
Codex and Cursor directory layouts. These are file-installation checks, without inference.

Repository checks passed: Cargo format, warning-denied workspace/all-target Clippy,
workspace tests and no-dependency workspace documentation, plus generated-document checks.
Cargo used the requested temporary target/environment and locked/offline mode. Workspace
tests needed the approved local-socket permission for their disposable loopback providers;
the first sandbox-only attempt was refused at socket bind. The completed run includes all
24 publication/parity regressions, including the F069 review regression. No real GitHub
transport was used. The review also re-ran the 47 Python/package regressions, generated-doc
check and warning-denied Clippy; the existing >=1.7.0,<2.0.0 installer compatibility tests
still pass without requiring the optional release-publication capability.

Reviewed operator-held `catalogue-pilot.json`, `first-use-pilot.json` and
`native-first-feature.json`: skills CLI 1.7.6 installed only the public skill pinned to
v1.7.1 (`0391aeb275f1e0d2e18842cff1e26c31000513e5`) in separate Codex/Cursor project fixtures.
Both selected targets used `.agents/skills/illumitech-forge`; fourteen resource hashes and
sentinels passed. The verified public binary passed doctor after a reviewed fixture commit
and the no-agent demo. One real Codex CLI greeting Feature then passed exact Hello+newline,
nine protected resource/lock/sentinel hashes, unchanged fixture HEAD, only the greeting and
three planning additions, independent final verification and status/checkpoint without replay.
The unexecuted earlier proposal was superseded before execution, not retried. These are
public v1.7.1 file-installation and CLI first-use observations; no new editor/model or
marketplace acceptance is inferred. The new installer remains a local candidate.

Pending operator gate: independently evaluate the final local candidate revision and prepare
its release metadata separately. The external F070 evaluator stays outside this source tree.
No directory submission, global installation or new publication is claimed by these task marks.

Subsequent operator candidate preparation updated release metadata to 1.8.0, added the
voluntary feedback form and generated setup documentation, and included F071 SQLite
maintenance. Final local workspace checks passed 314 Rust and 47 Python tests, plus
format, warning-denied Clippy, docs and three site tests. The exact candidate release
binary passed the eight publication, four compatibility and eleven installation cases.
Independent source/artifact binding remains in the operator-held candidate receipt;
publication and new IDE/platform acceptance are separate.

- [x] F070-05 Keep all seven first-party READMEs current, register the Workbench
  template and reject new unregistered app/crate READMEs. The user requested this as a
  continuing delivery rule; 48 Python/package regressions and generated-document checks
  passed before the authorized 1.8.0 publication commit. Historical notes and upstream
  third-party texts retain their original scope.
