# Recovering release publication

First run the selected executable's `--json contracts`. This recovery procedure requires
`contracts.release_publication=1`; version numbers alone do not establish the capability.
Published 1.7.0/1.7.1 lack this journal/protocol. If the field is absent or unsupported, preserve
local state and inspect provider releases, tags, inventory and downloaded bytes read-only.
Arrange manual operator recovery rather than repeating the nonrecoverable publisher or creating
a replacement release. Other skill routes remain compatible with Forge >=1.7.0,<2.0.0 and their
required contracts. The new protocol is a local candidate, not a claim about published releases.

`release publish` is an explicit, separately authorized operation. Keep its manifest, pinned
evaluators and state directory outside agent-writable source. Current passing evidence, a clean
evaluated commit, revision-bound human approval, matching GitHub remote and the existing evaluated
tag are required on every invocation. A passed execution, dry-run or version proposal does not
authorize publication. No account, model, permissions or credentials need to change for recovery.

Before the authorized first invocation, review the exact notes, SHA256SUMS, asset bytes, repository,
tag and expected visibility. Inspect command syntax with `release publish --help`. Use a local
preview, then the same command without `--dry-run` only within the reviewed authorization:

```sh
forge --state-dir /operator/state release publish /operator/release.yaml \
  --repository example/project --tag v2.0.0 \
  --assets /operator/assets --notes /operator/notes.md --dry-run
```

The default visibility is PRIVATE. Include `--expected-visibility PUBLIC` in both invocations when
the approved destination is public. Visibility is checked, never changed by this command. Dry-run
checks local inputs and gates only; it neither contacts GitHub nor starts a publication journal.

The journal uses the additive Store metadata key `release-publication:RELEASE_ID`, independent of
`draft-pr:RELEASE_ID`. It retains the immutable input contract, observed release ID, verified asset
IDs and pending create/upload/starter-delete/publish intent. Transport inputs and binary downloads
use fresh temporary directories beside the external state database. Source remains untouched.
Use `report` with the same manifest and state directory to inspect
`traceability.release_publication_journal`; this is read-only and records no approval.

An uncertain command response stops that invocation. Preserve the journal and repeat the exact
authorized publication command explicitly. Do not run implementation tasks or invalidate evidence
just to retry publication. Recovery uses complete GitHub release/asset collections and verifies
existing bytes from downloads before any new provider write:

| Interruption | Next explicit invocation |
| --- | --- |
| Draft creation | Reuse the matching draft; create only if no candidate has ever been observed and a complete collection proves absence. |
| Asset upload | Verify/reuse matching uploads; send only missing expected files. |
| Empty starter upload | Delete only the zero-byte starter covered by the pending upload intent for the observed candidate, then upload that file. |
| Starter deletion | Reconcile its asset ID; do not repeat a deletion already applied. |
| Publication | Verify all published asset downloads and record completion; do not publish again. |
| Completed publication | Repeat provider and byte checks with no remote writes. |

A matching pre-existing published release can be reconciled without a prior local publication
journal. Current source/human gates, the resolved remote tag, complete inventory and downloaded
bytes still have to pass; this creates local evidence with no remote writes.

Source identity always comes from the mandatory remote tag resolved to the approved commit,
including peeling annotated tags. GitHub's release `target_commitish` may be a branch such as
`main`: it is [unused for an existing tag](https://docs.github.com/en/rest/releases/releases#create-a-release).
Do not compare it to the approved commit or resolve a moving branch in place of the tag.

Notes, title, remote tag/source, visibility, unexpected assets, conflicting bytes, replaced/disappeared
verified assets and stale approval fail closed. Restore the exact reviewed local inputs only when
that is still the intended contract. If remote state differs, inspect it read-only and obtain an
operator decision for that specific conflict. This command does not overwrite notes, clobber assets,
delete a release, move tags or transfer old approval to a changed revision. Never edit SQLite or
evaluator files to bypass a block. If an already observed release disappears, recovery stops for
manual reconciliation instead of creating a replacement.

The source publication regressions use disposable Git and GitHub CLI transports, including invalid
UTF-8 binary bytes and uncertain outcomes before/after mutations. These checks establish local
recovery behavior; they do not attest to a real provider release or native-account acceptance.
