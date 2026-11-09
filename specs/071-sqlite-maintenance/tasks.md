# F071 tasks

- [x] F071-01 Inspect current primary advisory and narrow the dependency update.
- [x] F071-02 Update the two locked packages and regenerate notices/inventory.
- [x] F071-03 Pass the current audit and final workspace/publication/install gates.

The earlier full workspace run passed 314 tests before this maintenance update; it is
retained as historical evidence and does not substitute for final-source checks.

Final maintenance verification passed: 314 workspace tests, 47 Python/package tests,
format, warning-denied all-target Clippy and workspace documentation. The candidate
release binaries passed eight publication scenarios, four compatibility cases and eleven
installer scenarios, with actual binary-archive inclusion. The current OSV result covers
272 registry packages with no findings. Four extracted crates and their README bytes
also passed. These local checks do not authorize publication or new platform acceptance.
