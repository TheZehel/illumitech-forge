# Windows portability candidate (FORGE-014)

The published **0.2.0 remains macOS ARM64 / Linux x86_64 only**. This worktree
adds an unreleased Windows 11 x86_64, MSVC, PowerShell 7 candidate. Native CI evidence is tracked by revision; configuration alone is not acceptance. No authenticated Windows agent acceptance
has been performed on the macOS implementation host. Windows ARM64 and Windows
PowerShell 5.1 are not supported; scripts require PowerShell 7 and stop on 5.1.

## Inventory and contracts

| Unix assumption / location | Windows implementation / validation |
| --- | --- |
| `forge-adapters/process.rs`: process groups, SIGTERM/SIGKILL | Private, non-inheritable Job Object with KILL_ON_JOB_CLOSE, no breakaway. Atomic `CreateProcessW` membership via `PROC_THREAD_ATTRIBUTE_JOB_LIST`; explicit stdio-only `HANDLE_LIST`. Terminate entire Job on timeout/cancel/output overflow/normal parent exit/future abort. |
| `engine.rs`: external `kill -0` | Native OpenProcess + zero-time WaitForSingleObject. Access denied/unknown errors count as alive; never reconcile based on absence of permission. PID reuse conservatively blocks reconciliation. Unix uses kill(2), preserving EPERM as alive. |
| CLI Ctrl+C and daemon/publication SIGTERM | Ctrl+C on both platforms; Windows also handles Ctrl+Break. Native tests send Ctrl+Break through a separate controller attached to the child’s private console, never the test runner's group. |
| PATH names and Unix executable mode | Windows discovers `.exe` / `.com` in absolute PATH entries, or explicit native paths. No implicit shell, current-directory search or PATHEXT script execution. `.cmd`, `.bat`, `.ps1` wrappers fail with guidance; use native agent distribution or an explicit trusted `pwsh.exe -File` executor. |
| Argument quoting | Unix retains `Command::args`; Windows uses explicit application path and UTF-16 CRT argv quoting, without a shell. Native regressions include empty arguments, spaces, quotes, trailing backslash, Unicode, metacharacters and CRLF. Client model and permission arguments unchanged. |
| fs2 worktree locks / SQLite WAL | Existing cross-platform libraries retained. Core lifecycle tests run on Windows too: lock conflict, cancellation, pause/resume, approval, tamper detection, bounded retries, crash/reconciliation. No schema changes. |
| Revision Unix mode bits | Unix mode hashing retained under cfg(unix); Windows hashes bytes and paths. Fingerprints are platform-local evidence, not portable approvals. Canonical paths still enforce evaluator/state/artifact boundaries. Windows junction escape has a native test. |
| Tests using `/tmp`, sh/sleep/yes and executable shebangs | Unix adapter cases retained; Windows native adapter suite covers trees and output. Shared CLI tests use explicit Python native fixtures; test-only EXE shims replace shebang wrappers for Git/gh. Python/Rust are CI dependencies, not installed-binary dependencies. |
| Unix signal-only Git clean-filter test | Unix-specific case remains gated. Windows native publication cancellation and daemon cancellation use Ctrl+Break; manual interactive Ctrl+C acceptance remains pending. |
| tar.gz / Python packaging | Unix archives unchanged. Windows candidate ZIP includes EXE, MIT, dependency notices/licenses, PowerShell scripts/examples, internal SHA256SUMS and external archive `.sha256`. No release upload step. |
| Bash-oriented usage | `examples/windows/workflow.ps1` covers inspect, dry-run, run, status, pause/resume/cancel and spec import. No admin, WSL, Bash, service or Docker requirement. |

## Atomic containment design (FORGE-WINDOWS-ATOMIC)

The private Job uses `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, is non-inheritable,
and allows no breakaway. Stable Rust 1.94 calls documented `CreateProcessW` with
`STARTUPINFOEXW`, `EXTENDED_STARTUPINFO_PRESENT` and
`PROC_THREAD_ATTRIBUTE_JOB_LIST`. Membership is part of creation, eliminating the
previous create-suspended / assign-Job gap. There is no post-creation assignment,
thread enumeration, resume API, nightly API or parent-only fallback. Failure to
establish membership fails creation. Nested-job restrictions remain errors.

`PROC_THREAD_ATTRIBUTE_HANDLE_LIST` limits inheritance to stdin/stdout/stderr pipe
ends; neither the Job nor parent-side pipe handles are inherited. Stdin is EOF.
The application path is explicit; arguments use Microsoft CRT quote/backslash
rules and mutable UTF-16 storage, with NUL and command-line length rejection.
Cwd is UTF-16; the environment is inherited. No shell interpolation is used.
Attribute storage is aligned and released with `DeleteProcThreadAttributeList`;
all owned handles close on errors. The attribute values live through deletion.

The existing bounded drain and cancellation flow is retained. Windows reads the
anonymous pipes through Tokio file readers and polls the process handle with
zero-duration waits and a 10 ms async delay. Cancelling a wait leaves no detached
blocking process wait. On timeout, cancellation, output overflow, parent exit,
observer error or future abort, the Job terminates the tree. Hard supervisor
termination closes the Job in the OS. Windows cancellation is immediate, without
a POSIX TERM grace phase. Unix spawning and supervision behavior is unchanged.

Added native regressions check membership immediately after `CreateProcessW`,
then hard-terminate the supervisor before spawn returns (no Rust destructors),
and verify the child dies. Other regressions cover failed creation, input
rejection, unrelated inheritable-handle exclusion, observer failure cleanup,
stdin EOF and stderr bounds. Existing descendant, argv/Unicode/CRLF, cancellation,
crash/reconciliation and shared lifecycle coverage remains enabled.
These Windows-only regressions are source candidates, **not executed
Windows evidence on this macOS host**. They cannot inject a crash inside the
kernel API: atomicity there relies on the documented creation contract.

Sources: [Microsoft Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects),
[creation attributes: JOB_LIST and HANDLE_LIST](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute),
[CreateProcessW](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw),
[attribute-list allocation](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-initializeprocthreadattributelist),
[CRT argument parsing](https://learn.microsoft.com/en-us/cpp/c-language/parsing-c-command-line-arguments),
[atomic Job assignment](https://devblogs.microsoft.com/oldnewthing/20230209-00/?p=107812).

## Per-user installation

If you only have Windows PowerShell 5.1, download the official PowerShell 7
`win-x64.zip`, extract it into a directory you own (for example under
`$env:LOCALAPPDATA`), then launch its `pwsh.exe` before running these scripts.
This does not require an administrator or a machine-wide PATH change. Follow
[Microsoft's ZIP installation instructions](https://learn.microsoft.com/en-us/powershell/scripting/install/installing-powershell-on-windows#install-the-zip-package).
Do not bypass an organizational execution policy; use its approved script-signing process.

Download a reviewed candidate ZIP and obtain its SHA256 from the reviewed release
channel. A checksum proves integrity against that trusted value, not publisher identity.
Run the reviewed installer from the checkout (or verify the ZIP before extracting its script):

```powershell
$zip = '.\illumitech-forge-candidate.zip'
$expected = (Get-Content -LiteralPath "$zip.sha256").Split(' ')[0]
# Verify the sidecar itself against the reviewed release evidence first.
.\scripts\install.ps1 -Archive $zip -Sha256 $expected
# Optional: -InstallDir 'C:\Users\Example\Apps\Forge' -NoPath
.\scripts\uninstall.ps1
```

Default destination is `%LOCALAPPDATA%\Programs\IllumitechForge`. Only the user
PATH is changed, once, and only that added entry is removed on uninstall. Open a
new terminal afterwards. Repeated install/uninstall is safe. Different existing
files, unmanaged destinations, redirected paths and modified managed files are
preserved by refusing the operation. Upgrades require explicit uninstall of the
unchanged old installation. Unknown files survive uninstall. Do not point the
installer at an existing published 0.2.0 installation for this task.

## Remaining acceptance gates

- Run the committed Windows MSVC CI: build, fmt, Clippy, full shared suite, native
  Job/argv tests, ZIP checksum/install/uninstall/PATH/preservation checks.
- Run on real Windows 11/PowerShell 7 with authenticated native Codex, Git and gh;
  validate permissions/model, spaces/Unicode, Ctrl+C, timeout, hard crash,
  orphan reconciliation, filesystem preservation and isolated draft PR/resume.
- Confirm atomic creation and its boundary regressions in the separately run
  Windows CI. Native authenticated Windows pilot is explicitly deferred by the
  user; runtime acceptance remains pending.
- Memory remains optional. If enabled, verify MCP scope without Docker or duplicate
  hooks. Claude remains experimental and needs a separate authenticated pilot.
- Human approval must bind to the exact reviewed revision and Windows checksums.
  No Windows release, existing-artifact replacement or global installation is
  authorized by this implementation. Mock Git/gh success is not live acceptance.

## Validation scope

The implementation host is macOS ARM64 with Rust 1.94.0. Local formatting,
Clippy, 44 Rust tests, documentation and the ZIP inventory regression passed.
The fixture ZIP test alone does not prove Windows execution. Native Windows CI
runs the shared and platform-specific suites, builds the EXE and exercises
PowerShell installation, preservation and pilot preparation.

No authenticated Windows 11 pilot has been run. The published v0.2.0 and local
installation remain unchanged. No service or autostart is enabled.

## Deferred authenticated pilot

`scripts/windows-native-pilot.ps1` prepares a disposable repository with spaces and
Unicode, a pinned external evaluator and external state. It preserves existing
folders by requiring a new root. ai-memory is optional and no hooks are installed.

```powershell
# Preparation only: inspect and dry-run; no agent invocation.
.\scripts\windows-native-pilot.ps1 -Root "$env:TEMP\forge-native-review"
# After reviewing the generated manifest, use its exact paths:
illumitech-forge --state-dir "$env:TEMP\forge-native-review\state" release run "$env:TEMP\forge-native-review\native-pilot.json"
illumitech-forge --state-dir "$env:TEMP\forge-native-review\state" report "$env:TEMP\forge-native-review\native-pilot.json"
```

Alternatively, `-Run` opts into immediate execution in a new root. The pilot uses
the authenticated native client with its configured model and tests implementation,
read-only review, arithmetic behavior and file preservation. A separate CI smoke
checks preparation and positive/negative evaluator fixtures, without invoking an
agent. It does not satisfy the authenticated Windows 11 pilot, interactive Ctrl+C,
or live draft PR acceptance gates listed above.
