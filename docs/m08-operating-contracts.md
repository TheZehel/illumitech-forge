# M08 issue to reviewed execution to draft PR

FORGE-032 adds opt-in selected GitHub demand import and strengthens the existing draft-pr
publisher/journal. Local regression checks do not establish coordinator acceptance, a live
GitHub pilot or native platform support. Claude and Windows runtime acceptance remain deferred.
No credentials, account/model overrides, API inference fallback, workflow dispatch, issue
comment/status mutation, repository setting changes, force push, merge or automatic release
are part of this workflow.

## Selected demand

```sh
illumitech-forge --json issue import --repository example/project --number 42 \
  --root PROJECT --ops-root OPERATIONS --dry-run
illumitech-forge --json issue import --repository example/project --number 42 \
  --root PROJECT --ops-root OPERATIONS --source-json SNAPSHOT.json --dry-run
```

Preview is the default. Live import invokes exactly `gh issue view
https://github.com/OWNER/REPO/issues/N --json number,url,title,body,state` through the configured
authenticated gh CLI. There are no model calls or remote writes. The qualified URL pins
`github.com` regardless of GH_HOST/default repository. Returned identity must match the selected
host/repository/number exactly; redirects, query strings, alternative hosts and closed states
fail. Offline mode makes no gh call and validates the same identity from captured issue-view
JSON. Offline provenance is always `offline_snapshot` and `provider_attestation: false`, even if
the snapshot was originally captured from a live provider. Live provenance means authenticated
gh readback in this invocation, not a signed or permanent attestation.

A snapshot must be a regular nonsymlink file outside known credential paths. Input is bounded
to 128 KiB, body to 64 KiB and title to 200 bytes; title/body must be nonempty. Unsupported
states, malformed JSON, control characters, redacted/sensitive content and explicit permission
bypass/instruction override attempts fail without writing a candidate. Detection is conservative,
not a general prompt injection classifier: every accepted body remains untrusted data.
Labels, comments, authors and other payload fields are discarded. The body/title are inert JSON
inside an indented spec appendix. They never become a shell argv, model, permission, task ID,
install, CI contract or publication label. Generated task descriptions refer to the selected
identity; operator curation and external evidence are still required.

`content_hash` / `issue.content_sha256` is SHA-256 over compact serde_json serialization of
exactly `{body,number,state,title,url}` in sorted key order (UTF-8). JSON formatting, labels,
comments and snapshot/live provenance do not affect it; body whitespace does. IDs are stable
for repository + issue number: `032-issue-<first 12 SHA-256 hex of lowercase OWNER/REPO>-N`.

Add `--apply` to create the reviewed spec/plan/tasks and external manifest. Optionally supply
`--expected-content-hash HASH` from preview to reject source changes between review and apply.
The JSON response includes `issue`, `candidate_id`, `spec_id`, `content_hash`, `files` with exact
paths/content, `manifest` and `required_next_steps`. All files are preflighted before any write.
Identical application is a no-op; differing local edits or changed source are preserved and
cause a conflict. There is no automatic update/overwrite. Symlink/traversal protections reuse
guided creation. A filesystem race or I/O failure can leave a partial create-only candidate;
retry preserves existing files and creates only matching missing files.

`OPERATIONS` must be outside Git projects. The candidate uses read-only native execution,
configured default model, human gates, optional memory, Actions off and no automation/publish.
Import does not execute it. Review observable acceptance and external evaluator hashes, then
use `release inspect CANDIDATE` and `release run CANDIDATE --dry-run` before explicitly running
an operator-authorized execution contract. Evaluators and SQLite remain operator-owned.

## Additive source schema and traceability

Existing schema-version-1 manifests remain compatible. An optional `source` object has its own
version and strict fields:

```yaml
source:
  schema_version: 1
  provider: github
  repository: example/project
  number: 42
  url: https://github.com/example/project/issues/42
  content_sha256: <64-hex-digest>
  provenance: offline_snapshot # or authenticated_gh
  spec_id: 032-selected
```

The spec must be referenced by a declared repo. If publication is declared, its repository
must match the demand repository. Source metadata grants no permissions. Inspect, dry-run,
status/report and the workbench show `traceability`: issue, specs, task IDs, current local
revisions, recorded task evidence/evaluated revisions, independent review and the existing
publication journal. A current local commit is not claimed as implementation of the issue.
No commit/PR URL is invented before publication readback. Independent native review remains
advisory and never grants a human gate or transfers approval to a changed revision.

## Publication preview and CI authorization

```sh
illumitech-forge --json release draft-pr MANIFEST --dry-run
```

The local preview requires the existing explicit `automation.publish` target/branch contract.
It prints repository, effective checked remote, branch/base, actual configured human author
and committer, command argv, permissions, local preflight/evidence readiness, policy blocks and
traceability. It performs no provider read or write, native call, commit, staging or state
creation. Readiness may be false: a preview is not permission or acceptance. PR content uses
trusted generated release/spec/task identifiers and source URL/hash, evaluated commit/evidence
hash, manifest hash and independent-review status/hash; raw issue prose is never published.

Generated/default contracts keep `budget.actions: off`. Because push, pull_request and release
events can activate Actions, this policy blocks commit staging and remote writes. A skip-ci
commit marker is not a general guarantee and Forge never amends reviewed commits or changes
repository settings to evade CI. A separate operator-reviewed remote contract can explicitly
accept those CI side effects using both:

```yaml
budget:
  actions: on
automation:
  publish:
    repository: example/project
    remote: origin
    branch: feature/selected-demand
    base_branch: main
    draft: true
    ci_authorized: true
```

This M08 extension does not enable arbitrary workflow dispatch, certify provider quota, measure
CI costs or override an actions-off budget. `actions:on` without this separate CI authorization
still fails manifest validation. All individual task budgets retain actions-off validation.
`ci_authorized:true` with actions-off does not allow writes. `budget inspect` reports declared
policy separately from `provider_quota: unverified`; monetary/account balance remains unknown.
No such authorization was used against a live repository during implementation.

## Journal recovery and divergence

`release draft-pr` still uses the original `draft-pr:RELEASE` SQLite journal. It requires one
linked worktree on the declared branch, effective Git URL checks (including pushurl/rewrites),
human Git identity, current passing revision-bound evidence, including every declared human gate. Imported issue
candidates declare human gates. Legacy draft contracts can still use their declared external
gates without an additional mandatory human gate. Dirty
content is staged/committed only under the explicit publication/CI contract; a new commit is
reevaluated and requires renewed human approval. No approval is inferred from issue content,
reviewer prose or gh output.

Publication gh repository arguments are host-qualified as `github.com/OWNER/REPO`,
so GH_HOST cannot redirect an authorized write to another host. The CI authorization must
match the release target as well as the draft target. Before writes, query all-state PRs for the exact target/head. At most one open draft with exact
base/head repository/head branch/evaluated commit/title/body/URL is reusable. Ambiguous, closed,
merged, forked, modified or wrong revision PRs block without overwrite. Read the remote branch
before push: unexpected existing tips block; an already matching tip is reused. Push is ordinary,
never forced, followed by revision readback and another PR query. External concurrent changes
can still race a provider call; current local evidence and remote responses are rechecked, and
uncertainty never becomes success.

Before creation, persist `creating` with manifest/commit/body digest. Always query after create,
even when gh reports success. A provider timeout after creating the PR can therefore recover
through readback without a second create. A failed/unavailable/mismatched readback retains an
`uncertain` journal; retry queries first. If no matching PR can be verified after an uncertain
request or a previously published PR disappears, creation stays blocked for operator remote
reconciliation. It never blindly creates again or overwrites a changed PR. A verified entry
records URL, commit, source, evidence digest and `readback: verified`; ordered run events retain
the same linkage. Provider identity changes preserve the previous journal.

The workbench can inspect a publication manifest and this preview, but still refuses execution
or publication through its UI controls. CLI commands remain explicit. Merge/release require
separate operator actions and are never derived from a demand, comment, label or task.

## Validation boundary

`crates/forge-cli/tests/m08.rs` exercises selected offline/live-protocol import, preview/no writes,
provenance/hash consistency, application conflict preservation, malformed/injected demand,
CI/permission blocks, human evidence, readback recovery and divergence using disposable local
Git worktrees and gh/Git subprocess fixtures. Browser regressions in
`apps/forge-ui/tests/m08.spec.ts` use the embedded production UI, real local HTTP/CLI and the
native protocol fixture. These tests do not attest a real account, remote PR or platform.
A separately authorized live remote pilot belongs to the coordinator, not this implementation.
