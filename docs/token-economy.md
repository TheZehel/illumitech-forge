# Token measurement and bounded context (milestone 0.4)

FORGE-022 adds local measurement and an opt-in prompt assembly policy. Native comparison
and milestone acceptance belong to the coordinator's external gates. No percentage saving,
subscription balance, money conversion or quality equivalence is established by these changes.
The discovery proposal in [features/token-economy.md](features/token-economy.md) remains intact.

## Read measured usage

```sh
illumitech-forge --json --state-dir /tmp/forge-demo/state status /tmp/forge-demo/release.yaml
illumitech-forge --json --state-dir /tmp/forge-demo/state report /tmp/forge-demo/release.yaml --limit 20 --offset 0
illumitech-forge --json --state-dir /tmp/forge-demo/state report /tmp/forge-demo/release.yaml --limit 20 --offset 20
```

Both queries are read-only, including when state or the new tables are absent. They add
`token_usage` with optional `input_tokens`, `cached_input_tokens`, `output_tokens` and
`total_tokens`. Null means unavailable, never zero. Cache is included in input; total is
input plus output. Reasoning output is not added again. `coverage` is `complete`, `partial`
or `unavailable`. `observed` holds available counters from deduplicated observations, which
are lower bounds when calls are missing or interrupted; an entirely absent field is null.
Missing cache data can leave cache null while input, output and total remain known.

`task_token_usage` groups the journal by task. `report` adds `token_calls`, with task,
attempt, phase, declared model (null means native config unobserved), outcome, elapsed time,
usage units and limitations. Native thread/session and turn keys identify the observations.
Repo scope is hashed. Invocation identity includes reconciliation generation and attempt.
Counts include attempted native invocations with spawn errors, retries, failures, timeouts,
cancellations and earlier reconciled generations, regardless of acceptance. Old native
attempts without measurements remain unknown after upgrade. Command executors do not report
native model usage. Calls made by arbitrary commands or outside Forge are not observable.

Pages default to 100 records, accept 1–1000 and sort newest first. Offset applies to calls
and events separately, not to totals. Totals stream the entire journal with checked integer
addition and constant accumulator memory. Freeze the run when collecting multiple offset
pages: new events can move offsets. No journal rows are deleted for reporting. Per-task
attribution retains the first observation of a deduplicated unit; shared/resumed sessions
across tasks cannot establish separate task deltas. Forge does not automatically resume
native sessions. Use fresh sessions for controlled task comparisons.

State schema 1 gains additive `token_calls` and `token_units` tables only on a write operation.
Each invocation is journaled as incomplete before spawn. Reported usage is committed with a
run snapshot and event before post-execution revision/log/gate operations. Abrupt supervisor
loss leaves unknown consumption rather than fabricating completion. The existing orphan,
reconciliation and revision-bound approval rules still apply. Redacted stream captures are
retained per generation/attempt; see [execution-reliability.md](execution-reliability.md).

## Supported observations and limits

| Client/source | Accounting |
| --- | --- |
| Codex `exec --json` | Uses reported usage on terminal `turn.completed`/`turn.failed` or error events. Repeated session/turn completion counts once. When turn IDs are absent, explicit start boundaries supply invocation-local ordinals; without session IDs the invocation supplies the scope. |
| Captured Codex `thread/tokenUsage/updated` | Recognizes cumulative `tokenUsage.total` snapshots. Session totals supersede turn snapshots. It does not sum `last` with `total`, launch App Server or enable another transport. |
| Claude JSON `result.modelUsage` | Uses cumulative per-model totals once per session/model, including cache reads/creation in normalized input. Ignores main-loop `usage` when model totals are available. |
| Claude JSON `result.usage` only | Reports main-loop observations with explicit partial coverage; nested work may be absent. Missing cache components keep normalized input unknown. Claude remains experimental. |
| Missing, malformed, truncated, cancelled or crashed output | Preserves parseable observations, marks incomplete coverage, and keeps full consumption unknown. |

Repeated cumulative snapshots replace earlier values. Regressing counters and conflicting
turn duplicates retain the earlier observation and mark incomplete coverage. No proportional
attribution or subtraction from unrelated sessions is inferred. Provider-internal work not
included in native counters cannot be reconstructed. Planning, reasoning, tool loops and
compaction performed within a native invocation are included only to the extent that the
client reports them. The journal supports phase-labelled calls; this increment adds no
separate planning/summary model calls and does not infer their breakdown from prose.

Native protocol references consulted for implementation: [Codex non-interactive output](https://developers.openai.com/codex/noninteractive/),
[Codex App Server](https://developers.openai.com/codex/app-server/), and
[Claude usage accounting](https://code.claude.com/docs/en/agent-sdk/cost-tracking).
Installed-client and subscription behavior still require a separate native pilot.

## Preview context without invoking a model

```sh
illumitech-forge --json --state-dir /tmp/forge-demo/state context plan /tmp/forge-demo/release.yaml --task build --budget-tokens 4000
```

`context_plan` includes repo/task, sources with relative paths, provenance, SHA-256, byte and
token estimates, inclusion decisions, budget, mandatory estimate, status and limitations.
It does not expose file contents or read/query remote memory. This command plans bounded
assembly even if execution still uses the default `full` profile. `release inspect` and
`release run --dry-run` show the plan and argv for an explicitly selected bounded profile.
A blocked context plan exits 2. Inspection never creates or migrates state.

Add an optional manifest policy to select bounded execution:

```yaml
context:
  profile: bounded
  budget_tokens: 4000
  sources:
    - src/lib.rs
    - docs/contracts.md
```

This is an example contract; operators must review their actual manifest separately. A task
can specify its own complete `context` policy; it replaces the manifest policy for that task.
`profile: full` is the conservative default and preserves the existing native launch/prompt
behavior. Bounded mode keeps the same native executable, declared/default model, tools,
account, authentication, instruction loading and permissions. Command executors keep argv.

Mandatory context includes the whole task description, acceptance declarations, dependencies,
Forge safeguards, root AGENTS.md/CLAUDE.md when present, skills/core/SKILL.md when present,
manifest-referenced spec/plan/tasks, and AGENTS.md/CLAUDE.md governing selected file directories.
Instruction files explicitly listed as sources are always mandatory. They are not summarized
or trimmed to fit. Repeated paths are assembled once with combined provenance; optional
identical content is included at most once. Every source keeps a recovery path.

Optional files are considered in deterministic path order and included whole if they fit.
Otherwise the prompt keeps their path/hash for native tool retrieval on demand. Optional
memory already retrieved through the exact repo-to-project mapping fits only as a whole
untrusted historical payload; its omission is disclosed. No extra memory/model request is
made by the planner. Native/global instructions remain enabled and are outside the estimate;
when work moves to another directory, the native client must read its applicable instructions.
The planner does not scan unrelated project files, other worktrees or external instructions.

Only regular UTF-8 files under the canonical repo root are eligible. Traversal, symlinks,
nested Git projects, unreadable files and oversized files block assembly. Local resource
bounds are 100 explicit optional sources, 1 MiB per source, 8 MiB retained source text and
128 KiB assembled prompt. The numerical budget must be 1–1,000,000. A mandatory source that
cannot fit is retained in the plan as mandatory/included when readable, with an explicit
block; the native client is not invoked and no attempt is consumed. This is prompt assembly,
not a new OS sandbox or protection against malicious concurrent filesystem changes.

The estimate is `ceil(UTF-8 bytes / 4)` for the entire assembled prompt including references.
It is a disclosed heuristic, not tokenizer measurement or an inference cap. Tool retrieval,
provider state and native instructions may add tokens. No new model calls optimize context
by default or when selecting bounded mode.

## Reproducible comparison protocol

The coordinator should pin identical tasks, worktree revisions, external gates, native client
version, model/reasoning/configuration and permissions for both profiles. Use fresh isolated
sessions, paired repeated runs, recorded ordering and declared cache conditions. Collect
reports and raw redacted logs from both successful and unsuccessful attempts. Preserve
missing observations as missing, and record any external planning/summary calls separately.

Compare measured input/cache/output/total **per externally accepted delivery**, success rate,
number of rework attempts, and end-to-end latency (including planning and failed attempts).
Never discard failed samples. Do not use `run.status`, text, prompt byte estimates or synthetic
fixtures alone as proof of acceptance, quality parity or subscription savings. Define the
quality margin and savings target only after the measured baseline. The native paired
comparison and remaining milestone acceptance are deliberately external to this implementation.

## Candidate 1.0 scope

[Current guide](v1-candidate.md) records the exploratory pairs, including the latency reversal,
cache limitations and unknowns. Counts cover journaled CLI invocations/native turns, not
provider request counts or all account activity. Configured hooks, MCP/background work and
other sessions may consume resources outside that journal. Native configuration is preserved.
