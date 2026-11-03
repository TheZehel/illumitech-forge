# Public distribution preparation

This tree prepares candidate 1.3.0. The published release is still 1.2.0. The owner
requested local review and release preparation; publication requires the final
revision approval. The website remains a local preview. Packages use MIT;
native executable names are `forge` and `illumitech-forge`. Rust dependency aliases retain the existing
`forge_domain`, `forge_adapters` and `forge_catalog` imports. Package names are now:

1. `illumitech-forge-domain`
2. `illumitech-forge-adapters`
3. `illumitech-forge-catalog`
4. `illumitech-forge-cli`

Domain and catalog are independent; adapters depends on the exact domain version, and the CLI depends on all three exact versions. Publish domain before adapters. Directories
remain `crates/forge-*`. Recipes now live inside the catalog package at
`crates/forge-catalog/src/recipes.json`, so compilation never needs a parent checkout.
The four workspace packages target 1.3.0. Stable install examples continue to use
1.2.0 until publication. No upload receipt is claimed for this candidate.
Check the new version, ownership and authentication before a separately authorized upload.
Current candidate scope and native/platform limits are in [v1-candidate.md](v1-candidate.md).

## Current visibility — 2026-11-02

The owner opened the GitHub repository and a read-only provider check confirmed `PUBLIC`.
MIT remains unchanged. Public visibility does not publish this local 1.3.0 candidate,
replace source-bound acceptance, or authorize Actions, a website deployment or registry upload.
Future GitHub release operations must explicitly use `--expected-visibility PUBLIC`.
The dated 0.3.0 path below is historical; its private-repository premise no longer applies.

## Local preparation

Use the pinned Cargo 1.94 toolchain and Python 3.11+. Populate the locked dependency
cache first when network access is authorized. These checks then run offline:

```sh
python3 scripts/test_distribution.py
python3 scripts/distribution.py list
python3 scripts/distribution.py package
python3 scripts/distribution.py dry-run
python3 scripts/public_readiness.py > /tmp/forge-public-readiness.json
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

`list` checks all four inventories and their allowlists, metadata, dependency versions
and publication order. `package` creates Cargo archives and builds their extracted source
in a fresh temporary workspace, using patches to the extracted sibling packages. It
prints archive hashes and removes temporary build artifacts afterwards. Pass `--output FRESH_DIR`
to retain the verified crate archives and checksums. Dependency caches may
be reused, but extracted builds have their own fresh target directory. Package checks
allow dirty source for review; actual publication requires a clean approved revision.

Cargo 1.94 can assemble all unpublished workspace packages, but its automatic verifier
can fail with `no hash listed` in its temporary registry. Archive creation therefore uses
`--no-verify`; success is reported only after the separate extracted workspace build.
This is NOT proof of registry resolution, native execution, or upload. Patches never
modify the source manifests or the archives. The package tests remain in the archives;
workspace tests and native CI are separate gates.

`dry-run` additionally attempts `cargo publish --workspace --dry-run --offline --no-verify`.
Cargo may require registry/authentication network access even in dry-run, so failure is
reported as `pending_offline_registry_check`; local package/build checks still run.
A zero script exit means local checks passed, NOT that every reported gate passed.
No command in these scripts uploads. Once dependencies exist in crates.io, the operator
must separately run unpatched `cargo publish --dry-run -p NAME` in the order above,
without `--no-verify`, before authorizing upload.

Cargo's documented [packaging](https://doc.rust-lang.org/cargo/commands/cargo-package.html)
and [publish dry-run](https://doc.rust-lang.org/cargo/commands/cargo-publish.html)
semantics are the basis of these distinctions. Exact behavior above is for pinned 1.94.

Each crate includes README, MIT license, dependency inventory and preserved notices.
After regenerating `docs/dependencies.json` with `scripts/licenses.py`, synchronize each
crate's `LICENSE`, `THIRD_PARTY_NOTICES`, `THIRD_PARTY_LICENSES/`, and `dependencies.json`
from the repository copies. `distribution.py` rejects stale or missing copies.
The broad notice inventory intentionally includes target-specific and development
packages; it is not a claim that every listed dependency is linked into every crate.

## Public-readiness gate

`public_readiness.py` reads the current tracked and untracked nonignored tree, every
reachable Git blob from all LOCAL refs (including deleted content), CI permissions/action
pins and the locked dependency license inventory. It does not fetch remote refs or inspect
reflogs. Shallow history blocks readiness. Findings exit nonzero; outputs include hashes,
categories and byte counts, never matching content. It inventories current artifacts by
hash and flags sensitive paths and unexpected distribution/runtime files.

This heuristic is a first pass, not a secret-free certificate. A separate dedicated secret
scanner over complete fetched history, human review for private business data, license
obligations and binary/archive inspection remain mandatory. Keep full audit receipts and
operational evidence outside this checkout. Any unresolved finding blocks public opening.
Do not rewrite history as part of this task. Permissions stay `contents: read`; native
macOS/Linux and Windows candidate workflows remain intact. This does not grant Windows
runtime support to 0.2 or replace authenticated platform acceptance.

## Operator gates (not executed by preparation)

- Review the exact clean commit, packaged file lists, hashes, licenses, full-history scan,
  native CI and revision-bound human acceptance. Rebuild after any change.
- Verify each crate name, ownership, version absence and registry authentication at operation
  time. Confirm publishing scopes and approve upload separately. Never embed credentials.
- Check the actual GitHub visibility. If a visibility change is desired, it needs its own
  explicit approval and separate operation. Forge never changes visibility.
- Publish dependencies in the order above, waiting for each exact version to resolve before
  the CLI. On partial failure, query remote state before any retry; never overwrite versions.
- Preserve actual registry and GitHub receipts externally. Local dry-runs are not receipts.

`release publish ... --expected-visibility PRIVATE` is the default GitHub release contract.
`--expected-visibility PUBLIC` permits only an already-public target. Both require current
human-approved evidence, a clean evaluated commit, matching remote tag, and valid asset
checksums. Any visibility mismatch (including INTERNAL or missing data) fails before release
creation. The local `--dry-run` reports expected visibility and explicitly says the remote
visibility was not checked. GitHub release creation and crates.io upload remain distinct.

## Selected local publication path — 2026-10-30

The operator selected local verification followed by crates.io upload and then PR merge,
without another GitHub Actions run because the included compute quota is nearly exhausted.
The selected artifact is version 0.3.0. Local macOS gates and extracted-package verification
must be tied to the reviewed revision; earlier Linux CI remains historical evidence.
Windows is explicitly experimental. Its latest service-host correction has not passed a
native Windows pilot; this limitation must remain in the published CLI README.
The prior Windows failure and deferred gates are not reclassified as passing.

Use `[skip ci]` on the reviewed push and merge commit to avoid spending Actions compute;
do not bypass required branch protections or turn pending checks into artificial successes.
crates.io upload makes package source public even while the GitHub repository remains private.
Repository visibility is a separate operation. Cargo authentication and revision-bound human
approval still apply. This plan alone does not record a successful upload or merge.
