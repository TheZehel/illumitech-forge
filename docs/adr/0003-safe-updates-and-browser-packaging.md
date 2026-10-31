# ADR 0003 — Verified local updates, state recovery and browser packaging

Date: 2026-10-30. Status: implemented candidate; independent acceptance remains external.

Ship the React UI as static assets embedded in the Rust CLI and open it in the operator's
local browser. Do not add a desktop wrapper. The checked-in UI bundle is approximately
240 KiB (the locked build is authoritative); a wrapper would add another engine or OS webview,
packaging/signing/updater path, origin bridge and platform maintenance surface. No desktop
app, signed publisher identity or native feature parity is claimed. UI runtime MIT notices
for React, React DOM and Scheduler travel inside the CLI package.

The existing loopback-only HTTP origin, random local capability, HttpOnly session, CSRF
and native approval boundaries remain authoritative. Use the OS browser sandbox and native
client sandbox; embedding static assets grants no filesystem or shell bridge. The foreground
workbench lifetime owns the native session and a shared state maintenance lease. Stop it
before snapshotting. An installed CLI resolves assets from its own binary, never an ancestor
checkout or network CDN. Rebuild locked assets deterministically and compare their byte hashes.
The single CLI atomic replacement also replaces its embedded UI, allowing binary rollback
without a separately cached UI installation. Browsers receive the server's cache policy.

Updates are explicit-prefix and local-artifact only. Default preview checks a regular,
executable, matching thin native host binary, its caller-supplied SHA-256 and a bounded
`--version` probe on a private verified copy. No download, signing claim, implicit downgrade,
PATH rewrite, global install, hook or supervisor mutation occurs. Unknown installations require
separate operator reconciliation; never adopt them based only on their filename/version.
Verified immutable generations and receipts preserve the rollback chain; a journal makes
interrupted swaps recoverable with the same artifact. The caller checksum proves integrity
against that input, not publisher provenance. No update executes a migration.

Keep the SQLite state schema at 1 for this milestone. Existing schema-1 additive tables remain
readable; schemas other than 1 are rejected by snapshot/restore and versions greater than 1
are rejected by Store before schema writes. Future incompatible migrations must increment
`user_version`, require a matching tested snapshot and fresh-target restore before any explicit
binary downgrade, and implement their own migration/rollback contract. Recovered task evidence
is historical: current revision/evaluator checks still control execution and approval.

Snapshots hold a separate exclusive maintenance lease plus SQLite write reservations and use
SQLite's backup API, including WAL, for both the engine and workbench databases. Probe a copied
restore before making a snapshot visible. Restore verifies inventory, hashes and schema then
publishes a complete staging directory to a nonexistent target with no replacement. No live
operational state is overwritten. A failed/interrupted staging copy never becomes the target.
Unrecognized SQLite files, symlinks and special files fail closed. Snapshot contents may include
operationally sensitive logs/manifests; keep recovery artifacts in operator-controlled storage.
This is integrity checking, not encrypted backup or protection against a malicious OS account
that already has write access to parents, binaries, receipts or snapshots.

macOS is the current native validation host. Linux implements ELF host checks, atomic
no-replace recovery and the existing user-systemd interface but needs a native pilot. Windows
implements PE checks and the existing user-scheduler interface; it remains experimental until
a native pilot. macOS universal binaries and unsupported architectures fail closed. No Windows
runtime support is retroactively added to 0.2. Per-user service plan/apply/start/stop/remove
remain explicit operator interfaces, with no administrator login or auto-start side effect in
update or restore. No service was enabled by this milestone.
