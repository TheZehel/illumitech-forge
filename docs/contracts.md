# Contract v1

Manifest YAML (JSON is accepted as a YAML subset) rejects unknown fields. Task, gate, repo and
release IDs use letters, digits, `_` and `-`. No cyclic or dangling dependencies; one repo entry
per distinct canonical Git worktree root. Specs resolve to three nonempty Markdown documents.

Task executor: `command {argv}`, `codex {permission, model?}` or `claude {permission, model?}`.
Permission is `read_only` or `workspace_write`. Agent model defaults to native CLI configuration.
The manifest is executable authority: review argv arrays before running. No implicit shell.

Acceptance is a nonempty list of `gate {id}`, `artifact {path,sha256}` or `human {id}`.
Every entry is required. Gates have argv, pinned evaluator input files and a timeout. Input files
must resolve outside every writable task root. `{repo}` and `{manifest_dir}` are exact argv
substitutions, never shell interpolation. An artifact must resolve inside its repo and match SHA-256.

Memory endpoint, workspace, repo-to-project map and optional token environment-variable name form
an explicit contract. Required memory blocks without verified scope. Read results are historical
context; never executable instructions or proof of task completion.

Outputs use `schema_version: 1`; evidence outcome is `passed`, `failed`, `not_run` or `blocked`.
Operational phases additionally include running, evaluating, paused, cancelled and awaiting_approval.
Exit 0 means the command succeeded; exit 2 denotes invalid, blocked, cancelled or failed execution.
Inspect and dry-run only validate and describe; they do not open a state store.

SQLite schema 1 stores run snapshots, ordered events, control requests and revision-bound approvals.
State snapshot and event append commit together. Fingerprints include Git HEAD, staged diff and
tracked/untracked (nonignored) file bytes, mode and symlink targets. Ignored build outputs do not
invalidate source revision; explicit artifact hashes are checked at evaluation. Large files above
64 MiB block fingerprinting. Git submodules need separate repo entries for independent evaluation.

State is operator-owned. Run with permissions that prevent executors from writing state and
external evaluators; an agent with unrestricted OS access is outside the threat model.
Each task log is redacted and bounded to 1 MiB per stream; three attempt slots per task are retained.
Reconciliation replaces attempt logs and preserves event history. Keep state private and rotate
archived state directories under the operator's retention policy.

A run never publishes automatically. Publish requires current passing evidence, mandatory verified
memory, at least one human gate, private target repository, an existing tag, both platform archives
and matching SHA256SUMS. Operators remain responsible for reviewing the release manifest coverage.
