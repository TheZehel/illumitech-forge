# Per-user daemon services

`service` defaults to `plan`. Supply an absolute, existing executable and release queue,
plus an explicit absolute `--state-dir`. Resolve symlinks first: redirected paths,
Windows reparse points, control characters and operational directories inside Git
worktrees are refused. No elevated commands, shell expansion, passwords or environment
secrets are embedded in definitions. The executable is never inferred from PATH.

Example (replace paths with the reviewed external operator directories):

```text
illumitech-forge --state-dir /srv/forge-operator/state service plan --executable /opt/forge/illumitech-forge --releases-dir /srv/forge-operator/releases
```

Use the same arguments with `apply`, `status`, `start`, `stop` or `remove`.
`--definition-dir` optionally separates supervisor definitions and their operation receipts
from the operational state. Supply the same absolute directory on every command. On macOS,
launchd rejects plists on external volumes mounted with `noowners`; choose an ownership-enabled
directory under your user home for definitions while keeping the queue and SQLite on external
storage. Preview checks the selected paths without creating them; apply creates only the
definition directory. Never change volume ownership settings merely to install the service.
`--platform macos|linux|windows` permits cross-platform **preview**; control requires
its native platform. Windows paths may contain spaces and Unicode; quote them as
individual arguments in the invoking terminal. No shell is used by the service manager.

- `plan`: render and hash the definition without writing files or contacting a supervisor.
- `apply`: stage the definition under `state-dir/services`; never load, enable or start it.
  Repeated application is a no-op only when the exact definition matches.
- `start`: explicitly bootstrap launchd in `gui/<uid>`, link/start a systemd user unit,
  or register/run an interactive-token Task Scheduler task with least privilege.
- `status`: read native supervisor output and existing daemon metadata. Reports installation,
  paused/failed/active/stopped state and stale/missing heartbeat (30 seconds). Native output
  is retained because scheduler states differ by platform. No SQLite or service state is created.
- `stop`: boot out launchd, stop/unlink the systemd user unit, or end the scheduled task.
- `remove`: stop and remove only intact owned resources. Repeated removal is a no-op.
  Manifests, SQLite, historical logs and operation receipts remain untouched.

Definitions are intentionally outside login-discovery directories. There is no `enable`,
login trigger, linger setting, startup task or system service. `start` supervises the current
user session; it does not opt into starting at future logins. The Windows task has no triggers
and is not registered until `start`. macOS needs an available GUI user domain; Linux needs a
systemd user session; Windows needs the current user's interactive Task Scheduler session.
Unavailable sessions fail without escalation. Native Windows acceptance remains pending.

Ownership combines a stable ID derived from the state path, an argv SHA-256 marker and
byte-for-byte definition comparison. Windows also retains and compares the scheduler's
exported XML before control/removal. Linux refuses foreign unit links, loaded fragments
and drop-ins. launchd refuses a loaded job with a different source path. Modified resources
are preserved: restore the reviewed definition/configuration before retrying. This is
accidental-change protection, not isolation against a malicious process running as the same user.
Use the original arguments for uninstall even if the executable or release queue was removed.

Mutations serialize on an external operation lock and keep immutable intent/result receipts
beside the definition. On partial start failure, rollback removes only newly registered resources;
existing tasks and definitions are retained. A failed cleanup remains an error with external
receipts for reconciliation. `stop` then `remove` is also the explicit rollback procedure.
Do not remove the operator directory to uninstall a service.

The native definition executes Forge directly. On Windows it invokes an internal service host
that places the daemon in a private kill-on-close Job, retries crashes up to three total attempts
with ten seconds between attempts, and stops on a blocked-contract exit without retry. Stopping
the scheduled task closes the Job and terminates its descendants. No scheduler restart policy
multiplies the attempt budget. This Windows correction still requires a native CI pilot.
The daemon reuses its locks, pause/retry/reconciliation
contracts and signal handling. It passes an internal logging flag: tracing stderr and final JSON
stdout go to `state-dir/service-logs`, using the existing process redactor. Each stream has one
active file and two archives, each at most 1 MiB; a single oversized event is capped. launchd and
systemd discard their duplicate stdout/stderr sinks. Restart never clears queue failure latches
or approvals. Service receipts are operational evidence, not evaluator acceptance of a revision.

## Validation boundaries

`cargo test --workspace` includes simulated native control, partial failures, ownership refusals,
Unicode/quoting, no-write preview/status, path redirects and log rotation. It never registers or
starts a real service. `python scripts/test_service_fixtures.py` validates generated plist/XML,
uses `plutil -lint` on macOS, `systemd-analyze verify` on Linux and `CommandLineToArgvW` on Windows.
These are syntax/argument checks, not lifecycle acceptance.

The `Explicit disposable user service smoke` workflow requires a manual dispatch with its
approval input set to true. It exercises apply/start/status/crash/restart/pause/stop/remove with
an empty external queue and retains operational data in the runner temp directory. It does not
request root, enable linger or manufacture a missing user session. Missing sessions and failed
native checks leave that platform gate pending/failed, never simulated as passing. Do not run
`scripts/test_service_native.py` on a development host. The native lifecycle results below were collected separately from fixture tests; authenticated
Windows clients, provider operations and publishing are outside this lifecycle check.

Native contracts: [systemd ExecStart quoting](https://www.freedesktop.org/software/systemd/man/253/systemd.service.html)
and [Task Scheduler command interface](https://learn.microsoft.com/en-us/windows/win32/taskschd/schtasks).

Subsequent operator validation on 2026-10-29 passed the macOS lifecycle with an isolated
empty queue, including crash/restart, retained pause and state-preserving removal. An initial
external-volume ownership failure was fixed with `--definition-dir` and retested successfully.
See [runtime evidence](expansion-runtime-acceptance.json). The subsequent [native CI run](https://github.com/TheZehel/illumitech-forge/actions/runs/40972411607)
also passed macOS and Linux lifecycles at revision `52f0a2f`. Windows recovery remains under
validation. An optimized macOS executable passed from internal storage; an external-volume
executable stalled in the macOS loader before application startup. Keep the executable and
definition on internal user storage for that layout; do not bypass macOS privacy controls.

## CI budget controls

Automatic validation runs once per pull request; branch pushes do not create a second matrix.
Push validation is restricted to `main`, and newer revisions cancel older runs for the same ref.
The validation matrix has a 25-minute job limit. Native pilots have a 12-minute limit and a
`service_platform` selector, defaulting to Windows for manual dispatch; choose `all` explicitly
when all three lifecycle gates need renewal. These controls preserve failed/pending gates.
The next Windows correction remains local and unvalidated natively while CI usage is deferred.
