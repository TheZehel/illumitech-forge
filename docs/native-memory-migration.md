# Native ai-memory migration rehearsal

`scripts/memory_migration.py` is an operator tool, not a memory server or a Forge SQLite
adapter. Forge continues to use MCP. It never opens the Docker volume database, stops a
production process, changes client configuration, installs hooks, or executes production
transition commands. Python 3.11+ and macOS/Linux are required. Docker is needed only for
an explicitly applied inventory/backup; offline tests have no Docker dependency.

## Trust and storage boundary

Keep the plan, inventories, approval documents, receipts, backups and native runtime **outside
agent-writable repositories**, on an explicitly selected mounted external filesystem. The
plan itself contains operational paths and scope identifiers: never commit it. Use an operator
account and restrict this storage to that account. `--apply` uses a filesystem lock to serialize
this tool's processes; it cannot lock out arbitrary servers, host processes or another Docker
host. An operator must enforce exclusive maintenance ownership and disable restart controllers.
An inventory or a boolean supplied by an operator is not independent acceptance evidence.

The tool refuses overlapping paths, symlinks, existing restore/backup destinations, unexpected
writable mounts, restart policies, active containers sharing the volume, and unmounted storage.
Docker inventory includes image ID, volume/mount, lifecycle timestamps and environment **names**,
not environment values. Source configuration is preserved privately in the full backup; do not
print or upload it. A mounted filesystem is checked, but the operator must independently confirm
that it is the selected physical external storage, not a bind mount or the source volume's disk.

Failed operations preserve partial output for investigation and refuse to overwrite it. Select
fresh destinations after investigating a failure. Never automatically delete a failed backup.
Receipts bind to the complete plan digest and backup manifest; editing the plan invalidates them.
Receipt generation is local evidence, not release acceptance or authorization.

## Plan contract

Copy this structural example to external storage and replace every placeholder. It is deliberately
not an executable production example. Select an unused endpoint port distinct from production.
`version` is the exact stdout of the pinned binary's `--version`; `server_version` is its MCP
`serverInfo.version`. Obtain SHA256 from a separately trusted release/build artifact before use.
The tool checks the file hash before executing `--version`, with a disposable HOME/data directory
and a minimal environment. A hash does not establish provenance by itself.

```json
{
  "schema": 1,
  "storage_root": "/external-memory-volume",
  "backup": "/external-memory-volume/rehearsal-01-backup",
  "native": "/external-memory-volume/rehearsal-01-native",
  "receipts": "/external-memory-volume/rehearsal-01-receipts",
  "binary": "/operator-tools/pinned-ai-memory",
  "sha256": "REPLACE_WITH_TRUSTED_SHA256",
  "version": "REPLACE_WITH_EXACT_VERSION_OUTPUT",
  "version_args": ["--version"],
  "server_version": "REPLACE_WITH_MCP_VERSION",
  "container": "REPLACE_WITH_CONTAINER_ID",
  "volume": "REPLACE_WITH_NAMED_DATA_VOLUME",
  "data_mount": "/data",
  "database": "REPLACE_WITH_RELATIVE_SQLITE_FILENAME",
  "config": "config.toml",
  "endpoint": "http://127.0.0.1:49190/mcp",
  "production_endpoint": "http://127.0.0.1:49191/mcp",
  "tables": {"REPLACE_WITH_PAGE_TABLE": ["id", "workspace", "project", "body"]},
  "probes": [
    {"workspace": "fixture", "project": "one", "query": "unique-first-sentinel", "ids": ["first-id"],
     "counts": {"REPLACE_WITH_BRIEFING_COUNT_PATH": 1},
     "pages": {"first-id": {"path": "sentinel.md", "sha256": "REPLACE_WITH_BODY_SHA256"}}},
    {"workspace": "fixture", "project": "two", "query": "unique-second-sentinel", "ids": ["second-id"],
     "counts": {"REPLACE_WITH_BRIEFING_COUNT_PATH": 1},
     "pages": {"second-id": {"path": "sentinel.md", "sha256": "REPLACE_WITH_BODY_SHA256"}}}
  ],
  "isolation_evidence": "/external-memory-volume/native-isolation.json",
  "shutdown_evidence": "/external-memory-volume/native-shutdown.json",
  "transition_approval": "/external-memory-volume/operator-approval.json"
}
```

Inventory the exact schema on an **isolated copy**, not the live memory DB. Include projections
for all durable identities, scopes, page contents and other records that must survive migration.
Each projection records a count and an order-independent content digest. SQL identifiers are
quoted, not executed as arbitrary SQL. A changed schema may require explicit projection review;
never relax a failed comparison just to pass. The full file manifest also preserves corpus, Git
metadata, config and SQLite WAL/SHM files when present.

`counts` uses slash-separated object keys in the pinned server's briefing response. The MCP
verifier expects briefing `scope.workspace`/`scope.project` (additional resolution metadata is
allowed), query `hits[].id`, and read-page `page_id`/`body`, as observed on native 3.2.9. Confirm these
response contracts against the selected binary before real integration; incompatible versions
fail closed and require a reviewed adapter change. Pick two existing, disjoint, exact-query
sentinels (at most 50 hits), never create production fixtures for this task. Checks include each
scope's IDs, counts and body hashes, plus absence of the other scope's sentinel. Global preference
hits are not project hits. This is sampled MCP isolation, supplemented by complete selected offline
projections; it is not proof of every possible authorization path.

## Preview, backup and isolated restore

All phases default to preview. Preview validates the plan and binary checksum, but executes no
process, calls no endpoint, and writes nothing:

```sh
python3 scripts/memory_migration.py rehearse --plan "$MIGRATION_PLAN"
```

Run the following only against an explicitly selected rehearsal source. For production inventory
or backup, obtain separate operational authorization first; this development task grants none.
The tool does not stop the source for you. Have the operator stop writes and all writers, disable
restart policies and external restart automation, then keep exclusive ownership throughout copying.
It checks stopped state and shared-volume containers before and after `docker cp`, including lifecycle
timestamps. Those checks cannot exclude a host-side writer or a short-lived third-party container;
maintenance isolation is a mandatory operator prerequisite. Online copies are unsupported.

```sh
python3 scripts/memory_migration.py inventory --plan "$MIGRATION_PLAN" --apply
python3 scripts/memory_migration.py backup --plan "$MIGRATION_PLAN" --apply
python3 scripts/memory_migration.py rehearse --plan "$MIGRATION_PLAN" --apply
```

Backup copies the entire stopped data mount to the selected external location and records hashes.
Restore verifies the backup receipt and hashes, copies to a new native directory, checks SQLite
integrity and records identity/content projections. It renames the copied configuration to
`*.production-disabled`; the preserved original backup is never opened by a new server.
No native server is launched automatically.

## Native launch and MCP acceptance

The pinned version's CLI/configuration must be reviewed before launch. The historical v3.2.9
Docker migration in `memory-upgrade.md` does not establish a native server invocation. Do not
infer flags, invent a health URL, or use the host wrapper, which may select production defaults.
Use the upstream source/help for the exact checksum-pinned native binary; this delivery does
not assert that any particular upstream binary or launcher was exercised.

The operator separately exercised pinned native 3.2.9, restored an isolated snapshot, compared
offline integrity/IDs/page contents, completed the local cutover and observed a fresh client's
full lifecycle through scoped MCP. The old Docker writer and its dedicated Colima VM are stopped;
rollback data is retained. See [operator evidence](expansion-runtime-acceptance.json).
That migration used an external operational procedure; it does not claim this generic script's
entire workflow or every isolation declaration was exercised against production.

Prepare a reviewed native launch command in the operator's external supervisor, with explicit
apply/authorization. It must select only the new data directory and loopback endpoint; use an
empty HOME, minimal environment, a fresh configuration and no inherited credentials. Disable
all schedulers, auto-improvement/consolidation, watchers/backfill, external LLM providers and
production hooks using settings supported by that version. Enforce denied outbound network at
the OS sandbox/VM boundary, allow only local MCP communication, and prevent access to production
storage and client settings. Preserve model selection: do not substitute another provider/model.
If those controls cannot be demonstrated, **do not start the native server**.

The external supervisor/operator records `isolation_evidence` with the plan SHA256 (`canonical`
JSON: sorted keys, separators `,` and `:`), endpoint, `data_dir`, `binary_sha256`,
`single_writer: true`, `schedulers: false`, `external_llm: false`, `production_hooks: false`,
`egress_denied: true`. These declarations are a prerequisite, not a sandbox implementation:
retain independent supervisor/config/firewall evidence alongside them for human review.

```sh
python3 scripts/memory_migration.py verify --plan "$MIGRATION_PLAN" --apply
```

This performs MCP initialize, initialized notification, tool discovery, scoped briefing/search,
negative cross-scope searches and page retrieval. HTTP 200 alone cannot pass. Proxies and redirects
are disabled, responses bounded, credentials not inherited. Authenticated servers and nonstandard
transports require a separately reviewed integration; no auth bypass or production fallback exists.

Stop the rehearsal server through the external supervisor. Record `shutdown_evidence` containing
`plan`, `stopped: true`, `writers: []`. Then:

```sh
python3 scripts/memory_migration.py finish --plan "$MIGRATION_PLAN" --apply
```

The tool checks that the endpoint is closed, verifies the untouched backup, runs integrity checks
on the isolated native copy and compares all selected ID/content/count digests with the baseline.
It does not open SQLite while intentionally running a server. An empty port alone is not proof
of no writer: verify the supervisor/process inventory too. Restart only through the isolated launcher.

## Protected transition command export (not execution)

Production stays untouched by this tool. `stop-writes`, `cutover` and `rollback` only export
operator-reviewed argv arrays into exclusive external receipts; they never run the commands.
Even exporting requires `--apply`, a verified rehearsal, unchanged backup and a separate approval
bound to the plan and exact phase. There is no default container stop, client rewrite or hook install.

The approval document contains `plan`, `phase`, `operational_authorization: true`, and `commands`
(an array of argv arrays, never shell strings). For `stop-writes`, also require
`exclusive_operator: true`. Review and execute exported commands only in the separately authorized
operator workflow. Use those commands to disable capture/schedulers and stop the current writer;
prove no remaining writers before the final backup.

For final backup, select a **new plan and new destinations**, inventory again, run applied backup,
restore and all verification steps on the final snapshot. Do not reuse the older rehearsal receipt
as proof of the final snapshot. Expected counts/projections must reflect the final state.
For `cutover`/`rollback`, approval additionally requires `writers: []`, `final_backup_verified: true`,
`final_backup_manifest` equal to the backup receipt's manifest digest, and
`fresh_client_verified: true` (fresh isolated Codex session against the candidate, before switching
production clients). External review must bind these facts to current supervisor evidence.

```sh
python3 scripts/memory_migration.py stop-writes --plan "$MIGRATION_PLAN" --apply
python3 scripts/memory_migration.py cutover --plan "$FINAL_MIGRATION_PLAN" --apply
python3 scripts/memory_migration.py rollback --plan "$FINAL_MIGRATION_PLAN" --apply
```

Rollback additionally requires `new_writes: false` and `old_binary_compatible: true`. Restore a
**fresh copy of the preserved old-version final backup** with the old pinned binary. Never point
an old binary at the migrated directory. If native production accepted any writes, automatic
rollback is refused until an operator reconciles divergence, preserves both copies, and repeats
verification under a new plan. Rehearsal receipts do not authorize discarding new writes.

## Validation boundary

`python3 scripts/test_memory_migration.py` tests preview without subprocesses or writes, path and
checksum refusals, active writers, inventory drift, complete copy preservation, integrity failures,
partial restore preservation, version mismatch, locking, MCP identity/isolation and rollback gates.
It does not require Docker, a downloaded native binary, credentials or a live memory service.
Native launch/isolation, real Docker backup, native MCP/auth transport, full schema compatibility,
fresh Codex continuity, physical storage and production transitions remain separate pending gates.
