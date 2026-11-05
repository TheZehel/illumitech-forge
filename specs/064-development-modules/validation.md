# F064 — Local operational validation

Historical F064 milestone: local 1.5.0 with interactive terminal mode selection, feature
planning/execution, read-only context handoff and QA coverage planning.

## Checks

- Full workspace regression invocation passed: 288 tests, including all integration
  suites and domain tests, with physical temporary/build paths and local loopback.
- After adding the final F3 shortcut, the three terminal mode/render tests passed again.
- Formatting, Clippy with warnings denied and workspace documentation passed.
- Final binaries were exercised directly in an interactive PTY with a native protocol
  fixture: initial picker, Feature planning from plain text, preparation of consent
  without executing, QA planning from an absolute path, F3 switching, Conversation,
  unavailable-module rejection, and clean Ctrl+Q shutdown all passed.
- The initial Python PTY harness had macOS process-finalization problems and was
  discarded. The final UI result above comes from the direct terminal session.

## Real native feature pilot

A disposable Git project used the existing configured Codex client, without a model
or API fallback. One feature task created greeting.txt with the expected bytes; the
pinned external check passed and a preexisting unrelated file remained unchanged.
The workflow reported `passed`. This pilot covered the feature engine before the
final F3-only UI shortcut; that shortcut was then verified in the direct PTY session.
The installed binary subsequently read back the pilot's passed state.

## Local installation

The user-authorized local Cargo installation was updated from 1.3.0 to the source
candidate 1.5.0. Both entrypoints were installed from the validated dev profile
without debug info; this is a local candidate, not a newly published release.
The prior binaries and Cargo receipts were copied to the operator build directory
and verified by checksum before replacement. Existing PATH links were preserved.

Installed files match the validated artifacts byte-for-byte:

- forge SHA-256: a38266229f9c11bded284a82f852629b779b9da37e4ecd5af9261d0c6309c2fd
- illumitech-forge SHA-256: 68dd6917e2a4e32f19eff5d8e946bb3c9e0222ba4c08d789cacf66e24692d2f6

Version and module-catalog readback passed through the user's PATH.

## Boundaries

Feature and read-only Conversation are operational in the terminal. QA is planning
only. Refactor and CI/CD remain explicitly unavailable. Automatic context compaction
and corrective loops remain future work. The browser Workbench retains its existing
workflow interface; the new picker belongs to the terminal.

No project commit, push, release publication or production deployment was performed.
No product-release acceptance ledger was granted. The native pilot has its own
external workflow evidence; it does not attest to all future repositories or tasks.

Builds moved to a compact SSD target after the internal volume filled. Only a failed
incremental compilation cache was relocated, with its content preserved; no user data
or unrelated repository was deleted.

## Subsequent QA delivery

F065 supersedes the planning-only QA boundary above. Its external gate passed 11
behavioral scenarios plus fmt, Clippy, workspace tests and docs. A direct terminal
pilot verified preparation without execution, separate confirmation, one execution,
status without replay, F3 and clean exit. The source candidate is now prepared for
1.6.0; publication requires final revision-bound release evidence.
