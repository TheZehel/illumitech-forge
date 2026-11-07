# F067 validation

- Final workspace test run: 308 unit/integration tests passed, zero failed/ignored;
  workspace doc tests passed. Test profiles optimize only the SHA-256 dependency through
  Cargo command-line configuration, preserving production source behavior.
- The regression creates an unborn fixture and observes a missing revision. Doctor creates
  no commit. After the test initializes an empty commit, diagnosis passes. A separate Git
  probe failure remains unverified and does not suggest inventing an initial commit.
- Sentinel bytes are preserved; no inference or execution-state creation occurs.
- Formatting, Clippy with warnings denied and workspace docs passed on the final code.
- Documentation generation, portable skill structure and deterministic package tests passed.

The published 1.7.0 tag and registry/archive bytes are unchanged. This is a source patch
following that release; release/changes.json declares a fix against 1.7.0 for the next
version-policy calculation. It grants no new publication or host acceptance.
