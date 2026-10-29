# Windows portability candidate (FORGE-014)

The published **0.2.0 remains macOS ARM64 / Linux x86_64 only**. This worktree
adds an unreleased Windows 11 x86_64, MSVC, PowerShell 7 candidate. Windows CI
configuration is not a passing CI run. No authenticated Windows agent acceptance
has been performed on the macOS implementation host. Windows ARM64 and Windows
PowerShell 5.1 are not supported; scripts require PowerShell 7 and stop on 5.1.

## Inventory and contracts

| Unix assumption / location | Windows implementation / validation |
| --- | --- |
| `forge-adapters/process.rs`: process groups, SIGTERM/SIGKILL | Private, non-inheritable Job Object with KILL_ON_JOB_CLOSE, no breakaway. Spawn suspended, assign Job, resume primary thread. Terminate entire Job on timeout/cancel/output overflow/normal parent exit/future abort. |
| `engine.rs`: external `kill -0` | Native OpenProcess + zero-time WaitForSingleObject. Access denied/unknown errors count as alive; never reconcile based on absence of permission. PID reuse conservatively blocks reconciliation. Unix uses kill(2), preserving EPERM as alive. |
| CLI Ctrl+C and daemon/publication SIGTERM | Ctrl+C on both platforms; Windows also handles Ctrl+Break. Native tests send Ctrl+Break through a separate controller attached to the child’s private console, never the test runner's group. |
| PATH names and Unix executable mode | Windows discovers `.exe` / `.com` in absolute PATH entries, or explicit native paths. No implicit shell, current-directory search or PATHEXT script execution. `.cmd`, `.bat`, `.ps1` wrappers fail with guidance; use native agent distribution or an explicit trusted `pwsh.exe -File` executor. |
| Argument quoting | `Command::args`, no concatenation/raw arguments. Native regressions include empty arguments, spaces, quotes, trailing backslash, Unicode, metacharacters and CRLF. Client model and permission arguments unchanged. |
| fs2 worktree locks / SQLite WAL | Existing cross-platform libraries retained. Core lifecycle tests run on Windows too: lock conflict, cancellation, pause/resume, approval, tamper detection, bounded retries, crash/reconciliation. No schema changes. |
| Revision Unix mode bits | Unix mode hashing retained under cfg(unix); Windows hashes bytes and paths. Fingerprints are platform-local evidence, not portable approvals. Canonical paths still enforce evaluator/state/artifact boundaries. Windows junction escape has a native test. |
| Tests using `/tmp`, sh/sleep/yes and executable shebangs | Unix adapter cases retained; Windows native adapter suite covers trees and output. Shared CLI tests use explicit Python native fixtures; test-only EXE shims replace shebang wrappers for Git/gh. Python/Rust are CI dependencies, not installed-binary dependencies. |
| Unix signal-only Git clean-filter test | Unix-specific case remains gated. Windows native publication cancellation and daemon cancellation use Ctrl+Break; manual interactive Ctrl+C acceptance remains pending. |
| tar.gz / Python packaging | Unix archives unchanged. Windows candidate ZIP includes EXE, MIT, dependency notices/licenses, PowerShell scripts/examples, internal SHA256SUMS and external archive `.sha256`. No release upload step. |
| Bash-oriented usage | `examples/windows/workflow.ps1` covers inspect, dry-run, run, status, pause/resume/cancel and spec import. No admin, WSL, Bash, service or Docker requirement. |

## Containment design and limit

The Job handle is owned by the supervisor and is never inherited. Windows closes
it if Forge crashes, killing associated descendants. Assignment/resume failures
fail closed and kill the suspended child. Nested jobs are supported on the Windows
11 target; assignment failure is an error, never a fallback to parent-only kill.
Windows cancellation is immediate termination, not a graceful POSIX TERM phase.

There is a narrow crash window between CreateProcess(CREATE_SUSPENDED) and Job
assignment: an abrupt supervisor termination there can leave a **suspended** root
process, which cannot yet execute or spawn descendants. The native crash tests
cover the recorded/running state after assignment. This pre-assignment window is
not claimed solved: atomic PROC_THREAD_ATTRIBUTE_JOB_LIST creation is a remaining
hardening gate before operational acceptance. Rust 1.94's standard-library custom
startup-attribute API is unstable; this implementation uses stable Tokio spawning
and documented ToolHelp/ResumeThread APIs. Never silently resume an orphan.

Sources: [Microsoft Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects),
[AssignProcessToJobObject](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject),
[atomic Job assignment and suspended-process crash window](https://devblogs.microsoft.com/oldnewthing/20230209-00/?p=107812),
[Rust Windows CommandExt](https://doc.rust-lang.org/std/os/windows/process/trait.CommandExt.html).

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
- Close or explicitly resolve the pre-assignment crash window above before
  claiming complete orphan containment.
- Memory remains optional. If enabled, verify MCP scope without Docker or duplicate
  hooks. Claude remains experimental and needs a separate authenticated pilot.
- Human approval must bind to the exact reviewed revision and Windows checksums.
  No Windows release, existing-artifact replacement or global installation is
  authorized by this implementation. Mock Git/gh success is not live acceptance.

## Implementation-host validation and exact changed files

Host: macOS ARM64, Rust 1.94.0. Passed `cargo fmt --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
(44 tests), `cargo doc --workspace --no-deps`, and
`python3 scripts/test_package.py` (one fixture ZIP inventory/checksum regression).
Windows-only tests and PowerShell install tests did not execute on this host.
The packaging regression uses a labelled fixture EXE; it is not a Windows build.
No commit, push, release, service, installed-binary replacement or durable memory write.

Files modified or added for this task:

- `.github/workflows/ci.yml`
- `Cargo.lock`
- `README.en.md`
- `README.md`
- `crates/forge-adapters/Cargo.toml`
- `crates/forge-adapters/src/clients.rs`
- `crates/forge-adapters/src/process.rs`
- `crates/forge-adapters/src/process/windows.rs`
- `crates/forge-adapters/tests/windows.rs`
- `crates/forge-cli/src/daemon.rs`
- `crates/forge-cli/src/engine.rs`
- `crates/forge-cli/src/main.rs`
- `crates/forge-cli/src/publication.rs`
- `crates/forge-cli/tests/common/mod.rs`
- `crates/forge-cli/tests/lifecycle.rs`
- `crates/forge-cli/tests/parity.rs`
- `docs/windows-portability.md`
- `examples/windows/workflow.ps1`
- `scripts/install.ps1`
- `scripts/package.py`
- `scripts/test-windows-install.ps1`
- `scripts/test_package.py`
- `scripts/uninstall.ps1`
