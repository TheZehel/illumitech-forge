# Reviewed QA execution

QA executes operator-reviewed subprocesses against one Git worktree. It makes no native
model call and preserves the configured account, model and permissions. It neither
installs tools nor launches discovery scans. The observed checks provide bounded
evidence; even `passed` is not a universal security attestation.

```sh
forge modules qa discover --root /projects/application
forge modules qa prepare /operator/qa/request.json --root /projects/application --ops-root /operator/qa-operations
forge modules qa prepare /operator/qa/request.json --root /projects/application --ops-root /operator/qa-operations --apply
forge modules qa start 065-format-review --root /projects/application --ops-root /operator/qa-operations --approve-scope REVIEWED_DIGEST
forge modules qa status 065-format-review --root /projects/application --ops-root /operator/qa-operations
```

Discovery only reads known stack indicators (Cargo.toml, package.json, pyproject.toml,
go.mod). Suggestions identify their indicator source, leave availability/applicability
unknown and grant no coverage. A package script can execute arbitrary project code;
inspect it before freezing any request. No dependency installation or network scan is
implicit. A reviewed command can itself use the network: review its argv and external
script contents for the intended permissions and effects.

Preparation previews by default. `--apply` creates candidate.json, request.json and
scope.json in `OPS/qa/ID`, outside every Git project. Files are create-only: different
content under an existing ID is rejected, preserving prior files. The candidate binds
the canonical worktree, Git directory, commit and nonignored source fingerprint, exact
request, Forge executable identity/hash and pinned external inputs to `scope_digest`.
Review those inputs' contents and every command before confirming that digest.
Preparation executes no QA checks. State and evidence must be protected from implementers
by the operator's filesystem permissions; a digest is not a substitute for those permissions.

The strict JSON request has `schema_version: 1`, a safe `id` (letters, digits, `-` or `_`,
up to 60 bytes), a nonempty one-line `title` (up to 200 bytes), `level`, and 1–32 `checks`.
Unknown fields are rejected. Each check requires a unique safe `id`, `coverage_id`,
nonempty `argv` array (up to 64 arguments, 8192 bytes each), `timeout_seconds` (1–600),
and `inputs` (1–64 absolute external regular file paths mapped to SHA-256). Inputs are
bounded at 256 MiB each. Traversal and symlink components are rejected for inputs and
operational paths, including at start/status. `format` defaults to `text`; `sarif` is
the other accepted value. Executables and scripts selected as evaluators should be
included in `inputs`; a pinned unrelated file does not prove a command's semantics.

Only an entire argv argument equal to `{repo}` is replaced by the canonical worktree.
Embedded placeholders are literal. There is no shell interpolation or inferred shell.
An explicit `sh -c ...` is operator-reviewed code with the same authority as any other
reviewed subprocess. It must not be treated as harmless prose.

| Level | Required coverage IDs |
| --- | --- |
| light | QA-QUALITY, QA-SECRETS, QA-DEPENDENCIES |
| standard | light plus QA-SAST, QA-AUTH, QA-BUILD |
| intensive | standard plus QA-ADVERSARIAL, QA-THREAT |

Coverage IDs outside the selected level or unknown to Forge are rejected. Each result
contains all eight coverage rows: unexecuted required coverage is `not_run`; coverage
outside the selected level is `not_requested`. All checks assigned to a coverage row
must pass for that row to pass. A missing executable is `unavailable`. Missing tools
and gaps produce overall `degraded` and exit 2, never success. Timeout, nonzero exit,
output overflow, invalid SARIF, scanner errors or findings produce a failed result
(exit 2). Source/contract mutation, cancellation or uncertain execution blocks acceptance.
Only every coverage row at the requested depth passing allows overall `passed` (exit 0).
A label and exit zero from an unrelated command do not establish meaningful security
coverage: the operator reviews relevance and the external acceptance gate remains independent.

The [request example](../examples/qa-request.json) deliberately covers only formatting.
Copy [the script](../examples/qa-quality.sh) to the external path in that request and
verify its SHA-256 after any edits. It uses an already-installed Cargo/rustfmt; no
scanner is installed. It leaves secrets/dependency coverage `not_run`, so even clean
formatting returns `degraded`. Add reviewed, pinned, already-installed tools to fill
those gaps; do not relabel formatting as a secret or advisory scan.

Start holds the existing worktree lock through execution, revalidates scope/source/
identity/input hashes before any spawn, and writes durable intent first. It uses the
existing subprocess supervisor with closed stdin, a process group, timeout, cancellation,
and bounded stdout/stderr (1 MiB per stream) with existing redaction. Output overflow
cannot pass. Ctrl+C/SIGTERM cancels CLI checks. No native tasks, repairs, publishing,
installation or automatic retries are performed by this module.

These subprocesses are **not filesystem read-only**. Builds may create ignored artifacts.
Forge fingerprints tracked and nonignored source before and after checks; detected
changes block acceptance and stop further checks. It never restores, deletes or
supersedes user changes. This detects observed source changes, not transient writes
that a command reverses itself, and does not impose an OS sandbox or constrain all
possible subprocess side effects. Review commands accordingly.

For `format: sarif`, stdout must be complete SARIF 2.1.0. The existing offline parser
retains findings and scanner errors with its URI/path checks; it never loads remote
references. Findings, malformed data, unsuccessful invocations and tool warnings/errors
cannot pass even if the executable exits zero. Evidence retains redacted output, parsed
findings and diagnostics; missing results are explicit, never inferred clean.

Execution intent, per-check intent/observations, result and hash receipts stay under
`OPS/qa/ID`. The existing Store/journal records QA events without new ad hoc tables.
Each check links an evidence file and SHA-256; status verifies retained result/evidence, final closed-journal and intent hashes,
source, contract, evaluator and input hashes without executing commands or writing state.
It never opens SQLite, preventing even WAL/SHM creation by a read-only connection.
A missing completion receipt or unsealed WAL changes leave the outcome blocked/unknown.
Stale source/inputs/evaluator yield `stale` with `effective_status: blocked` and exit 2;
a changed contract or evidence fails closed. Historical results are not fresh approval.

A second start never replays checks. In-progress or interrupted intent without a final
result is explicitly blocked/unknown, with any completed check evidence retained. Inspect
processes and source before preparing a new ID and reviewing a fresh digest. Do not edit
SQLite or remove intent to obtain a replay. There is no automatic resume command for QA.

## Terminal

F3 or `/mode` selects QA, Feature or Conversation; Refactor and CI/CD stay unavailable.
In QA mode, a REQUEST JSON path routes to `/qa REQUEST`. Enter prepares the external
candidate, displays exact argv, source, pinned inputs, coverage/gaps and digest, and
runs nothing. Enter on this prepared panel puts `/qa-run DIGEST` in the editor; another
Enter confirms execution. `/qa-status [ID]` inspects evidence for the selected project.
QA approval cannot use a Feature digest, and `/run` cannot approve QA. Changing mode or
preparing another scope clears both selected authorizations. Checks run asynchronously
while the terminal renders; Esc/Ctrl+C cancels a pending QA run (close an open panel
first), and leaving the terminal requests cancellation and bounded shutdown.

Bare legacy JSON/YAML manifest paths, `/qa light|standard|intensive MANIFEST`,
and `modules qa-plan MANIFEST --level LEVEL`
retain the legacy planning-only contract. They grant no execution consent. Feature
commands and F3 keep their existing routing. The browser UI is unchanged.
