# From request to verified delivery

Available in 1.2.0. Building or updating an installation does not change PATH order; verify the selected executable.

## Start

Run `forge` in a Git project. Choose Execute a task in Workbench or enter `/plan REQUEST` in the terminal. Native planning is read-only and cannot execute the proposed contract.

Review the objective, tasks, planned files, exact commands, external regressions and permissions. Saving prepares an external candidate only. Confirming its digest grants scope authority, never future completion acceptance. Native permission requests remain independent.

Use `/run DIGEST` to start, `/tasks` to list, `/tasks ID` to diagnose, `/report ID` for evidence and `/tasks resume ID` to resume. `/resume` still resumes conversations. Saved workflows are selectable after a Workbench reload.

## CLI

```sh
forge workflow doctor --root .
forge workflow recipes --root .
forge workflow prepare request.json --root . --ops-root /external/operation
forge workflow prepare request.json --root . --ops-root /external/operation --apply
forge workflow start 001-fix --root . --ops-root /external/operation --approve-scope DIGEST
```

Requests require schema_version, id, title, request and recipe. Checks contain explicit argv arrays. When omitted, existing project scripts or Cargo checks are suggested. Review watch scripts and select single-run commands. New workflows default to bounded context with a 16,384-token heuristic budget; mandatory instructions are never silently discarded.

## Recipes

Feature implements a bounded change. Bug requires a frozen external regression with a nonzero before_exit. Supply regression source through files and reference a full `{checks}/regression.js` argument. `{repo}` is replaced only as a complete argument.

Dependencies requires a rescan:true check (OSV-Scanner or Semgrep producing SARIF; Semgrep requires --error) and project tests. It accepts selected SARIF 2.1.0 findings from OSV-Scanner or Semgrep. Add a reviewed rescan and project tests. Findings and suggestions are untrusted data, never executable authority. Disappearance alone does not prove remediation.

PR prepares a local diff, validation results and description. Commit, push, PR creation and publication remain separate actions.

## Recovery and privacy

Scope consent is separate from revision-bound human acceptance. External gates determine completion; agent text never does. Existing manifests retain their contracts.

Changed contracts require fresh scope consent. Source reconciliation is explicit and invalidates previous evidence. Interrupted preparation or baseline checks before the execution checkpoint remain blocked: inspect the journal and files before preparing another workflow. Unknown process outcomes never count as success.

Memory is optional, hooks are not installed, and native account/model/permissions are preserved. This site cannot access local Workbench sessions and collects no code or telemetry.

Successful planning receipts linked by the owning server enter the execution token journal. Failed/unlinked planning and other sessions remain outside that total and are disclosed.

## External scanner with Homebrew installations

Frozen verifiers live outside Git trees. Homebrew may track its own prefix in Git, so a rescan executable installed there is rejected. For Semgrep, select a reviewed copy of its entrypoint in your external operations directory while preserving the existing runtime:

```sh
FORGE_OPS=/external/operations/path
mkdir -p "$FORGE_OPS/scanner"
cp -p "$(command -v semgrep)" "$FORGE_OPS/scanner/semgrep"
shasum -a 256 "$(command -v semgrep)" "$FORGE_OPS/scanner/semgrep"
```

Check both hashes and use the copy's absolute path as the check's first argument. Forge pins that file; it does not install or upgrade Semgrep. The copied entrypoint still uses the original Homebrew runtime: its hash does not attest every scanner library. Review the version and prepare a fresh contract if the installation changes.
