# M09 safe installation and recovery

FORGE-033 implementation: `installation.rs`, `snapshots.rs`, Store maintenance leases,
workbench/session lifetime leases and `tests/m09.rs`. Architecture is recorded in
[ADR 0003](adr/0003-safe-updates-and-browser-packaging.md). These are public implementation
contracts, not an external acceptance ledger.

## Diagnostics and update

`illumitech-forge --json installation doctor` is read-only. It reports the running path,
version, host, ordered PATH directories and candidate Forge binaries, including symlinks and
unresolved links. Competing binaries are never executed. Candidate provenance stays unknown.
It never creates the selected state directory or dumps unrelated environment variables.

```sh
illumitech-forge installation update --artifact ./release/illumitech-forge \
  --sha256 SHA256 --prefix ./local-install --dry-run
illumitech-forge installation update --artifact ./release/illumitech-forge \
  --sha256 SHA256 --prefix ./local-install --apply
```

Omitting `--apply` is preview. Paths must have no symlink components or parent traversal;
macOS operators should use canonical temporary paths (for example `/private/tmp`, not `/tmp`).
The prefix is exact: target is `PREFIX/bin/illumitech-forge` (`.exe` on Windows). The prefix
may be absent. The artifact must be executable, at most 512 MiB, a thin native host binary,
and report exactly `illumitech-forge MAJOR.MINOR.PATCH` in a five-second probe with bounded
output. Stable three-component releases only; no universal Mach-O or prerelease versions.
The checksum is supplied by the operator, with publisher identity/signature explicitly unknown.
No unchecked remote bytes are executed or downloaded.

An existing target must match `.forge-current.json` and the verified archive chain under
`.forge-installations/SHA256/`. Unknown targets, wrong hashes, foreign platforms, failed probes
and damaged archives fail before replacement. A prefix file lock serializes updates. Preview
returns exact target, current receipt, backup and interruption recovery action. Apply stages
and syncs the artifact, writes `.forge-update.json`, atomically replaces the regular target,
then publishes the receipt and removes the journal. Immutable generation receipts preserve
source path, SHA-256, version, platform, previous hash and timestamp. Applied operations, including rollback/recovery, append receipts in `.forge-update-history/`. The previous binary remains
available in its verified generation. Recovery checks both sides of an interrupted swap and
requires the same artifact/hash. A leftover journal after receipt publication is idempotent.

An explicit `--allow-downgrade` allows operator rollback. Stop tasks/services, restore the
matching snapshot into a fresh state directory and use that directory before running the older
binary. The updater never opens, migrates, restores or deletes operational state. It cannot
prove that a manually selected older binary supports an independently selected future state.
Every supported Store reader rejects unknown SQLite schema versions. Up to 128 archive-chain
entries are checked; archive pruning requires a separately designed operator workflow.

## Consistent snapshots and fresh-target restore

```sh
illumitech-forge --state-dir ./operator-state state snapshot --output ./snapshot-001
illumitech-forge state restore --snapshot ./snapshot-001 \
  --target-state-dir ./restored-state --dry-run
illumitech-forge state restore --snapshot ./snapshot-001 \
  --target-state-dir ./restored-state --apply
```

Use canonical paths; snapshot output/restore target must not exist, their parent must exist,
and they must be separate from source state/snapshot. Relative paths are resolved against cwd.
Restore preview performs no destination writes. Snapshot metadata includes schema/tool/minimum
reader versions, original location, time, manifest references, and all database/artifact hashes.
SQLite backup API captures committed WAL pages. Maintenance ownership excludes Store writers,
workbench sessions and reserved native calls; daemon ownership and SQLite's own writer lock
are also checked. Active/unreconciled run/task/native records block snapshot. Stop the workbench
and daemon, and explicitly reconcile orphaned tasks before retrying. Unknown SQLite files,
links, special files, more than 100,000 artifacts or more than 4 GiB of artifacts fail closed.

Snapshot creation exercises a separate restore copy, SQLite integrity and the actual Store
reader before success. Restore checks exact inventory (including unexpected files), hashes,
supported engine/workbench schemas and database integrity before staging. Recovery databases are sealed in SQLite DELETE journal mode, so preview never creates WAL sidecars. It verifies the copy again and
publishes to a fresh target only. Existing directories, even empty ones, are refused. The
source remains intact; no snapshot carries fresh acceptance or bypasses revision checks.
Schemas newer than supported remain unreadable even if their bytes have a matching hash.
There is no incompatible schema migration in M09. Schema-1 compatibility has local regressions
covering historical metadata and current WAL/workbench state; this is not proof against arbitrary
future schema-1 changes. A future incompatible change must increase the schema version.

## UI package, platforms and services

`npm ci` with the lockfile, `npm run typecheck`, `npm run test -- --workers=1` and
`npm run build` are the frontend checks. The supported local validation runtime is Node 24 LTS.
The build embeds deterministic `ui-dist` files and runtime license notices into the CLI crate.
A copied installed binary serves the workbench without its source tree or Node installation.
No desktop wrapper or native desktop feature parity is claimed.

Existing `service plan --platform macos|linux|windows --executable PATH --releases-dir PATH`
uses explicit `--state-dir`, with optional `--definition-dir`. Rendering is offline; application
and start/stop/remove are separate explicit actions limited to the native user supervisor.
Do not run those actions during update/snapshot/restore. They never require administrator
privileges or an automatic login. macOS local native checks are reported separately from
Linux/Windows pilots, which remain pending. Windows stays experimental.

No remote discovery, updater daemon, signed supply chain, encryption, native desktop wrapper,
global installation, service activation, client account/model changes or publication is part
of this implementation. Native/remote acceptance is owned by the coordinator; protocol and
browser fixtures only support local regressions.
