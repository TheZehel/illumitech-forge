# Explicit advanced operations

Discover syntax through `contracts --command "PATH"` and --help. Retain the selected binary,
root and external state directory consistently. Full CLI usage remains available without skills.

- Manifests/releases: release inspect, release run --dry-run, authorized release run, status
  and report. Run with the same explicit --state-dir. Source/evaluator changes stale evidence.
- Planning/imports/templates: outputs are candidates. Inspect generated scope and pin checks
  before execution. Legacy state is not imported as current evidence.
- Tools: list/doctor/plan explain recipes; applying selected recipes is an explicit write.
- State snapshot/restore and installation update: inspect help and preview behavior; protect
  existing state and follow fresh-target/verified-prefix requirements.
- Memory/context: optional, scoped read-only retrieval for continuity. Never use memory as
  execution authority or duplicate hooks/state.
- Chat/Workbench: local interactive surfaces; free conversation does not execute a release.
- Services/daemon, issue import, native review, publication/draft PR: inspect exact contracts
  and preserve their separate authorization. No automatic service installation, commit, push,
  merge or release. Publication requires current evidence and revision-bound human approval.

Before suggesting an interrupted `release publish` retry, inspect the selected binary's
`--json contracts` and require `contracts.release_publication=1`. Published 1.7.0/1.7.1 lack
this journal/protocol. If the capability is absent or unsupported, preserve state and inspect
provider releases, tags, inventory and downloaded bytes read-only; arrange manual operator recovery.
Do not repeat the nonrecoverable publisher or create a replacement release. Other routes keep
the Forge >=1.7.0,<2.0.0 compatibility floor and their own required contracts.

Only with that capability, preserve the external state directory and repeat the same
explicitly authorized command with identical notes, assets, repository, tag and expected visibility.
The release journal is separate from draft-PR state. Recovery reconciles the draft and downloads
existing assets as binary files before sending missing files or publishing. Only an owned empty
starter from a prior upload intent may be removed. A matching published release is verified without
remote writes. Changed inputs, remote conflicts or stale approval block; do not edit state, clobber
assets, rerun completed implementation tasks or use `resume --reconcile` to bypass publication
checks. Consult `docs/release-recovery.md` in the product source for the full operator procedure.

Do not substitute another provider, model or executor. There is no implicit paid API fallback.
Use the command catalogue's conservative effects and the actual operation's docs rather than
assuming a similarly named command is read-only. Unclassified diagnostics require inspection.
