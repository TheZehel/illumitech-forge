# Illumitech Forge Workbench 1.8.0

React/TypeScript/Vite frontend for the same-origin Rust bridge. No browser access to
native credentials or arbitrary projects. Native configuration and managed authentication
stay in the installed Codex client; no paid API fallback exists.

```sh
npm ci --ignore-scripts
npm run typecheck
npm run build
cd ../..
cargo build --locked -p illumitech-forge-cli
```

`npm run build` copies production assets and dependency licenses into
`crates/forge-cli/ui-dist`. That directory is included in Cargo source packages;
`build.rs` embeds only crate-local assets. Cargo builds and installed binaries never
read the parent checkout or require Node at runtime. After any UI edit, rebuild assets
before building the CLI. Committed/prepared assets are sufficient for Cargo consumers.
Vite's development server alone is not an authorized bridge origin; use the embedded
build for integrated tests and normal operation.

```sh
illumitech-forge --state-dir EXTERNAL_STATE workbench --root ABS_GIT_ROOT --port 0
```

Open the startup URL locally. Its fragment bootstraps an HttpOnly, SameSite=Strict cookie
and CSRF header, then is removed from browser history. Treat that URL as a local capability.
Closing the terminal/Ctrl+C stops the foreground bridge. No browser opens automatically.
`--manifest EXTERNAL_MANIFEST` opts into read-only Forge state/estimates for exactly this
project; absent/incompatible state stays unavailable. Sessions are only those created by
this workbench; saved threads require explicit resume. No other client history is imported.

`npm test` uses `@playwright/test`, Chrome's installed `chrome` channel, a temporary loopback
server and `tests/fixtures/workbench-native.py` from the CLI package. It makes no inference
calls and removes only its own temporary project/state. Build the CLI first. No browser
installation, profile access, global package, hook or service is required.

The protocol fixture is regression coverage, not authenticated account or milestone
acceptance. The external coordinator owns those checks. Limits and wire contracts are in
[the milestone documentation](../../docs/workbench.md).

Runtime React, React DOM and Scheduler are MIT; their full licenses ship in the embedded
`THIRD_PARTY_LICENSES.txt`. TypeScript (Apache-2.0), Vite (MIT), Playwright (Apache-2.0) and
Node/React type declarations are build/test tools only. Exact transitive packages and
integrity hashes are recorded in package-lock.json; upstream license files remain installed.

## Current release and skill integration

This frontend is distributed with Forge 1.8.0; see the [current release notes](../../docs/release-v1.8.0.md).
Forge uses the existing native Codex account/model. The optional [project skill](../../docs/skill-getting-started.md)
guides the assistant through the CLI; Codex CLI and its extension in VS Code/Cursor
use the same `.agents/skills` loader. Installing files does not prove model execution.

The CLI supports verified skill installation and [release-publication recovery](../../docs/release-recovery.md)
when the reported capability is present. Those operations keep their explicit scope,
external evidence and approval requirements; opening Workbench does not trigger them.
A voluntary [feedback form](https://github.com/TheZehel/illumitech-forge/issues/new?template=forge-feedback.yml)
collects reproducible setup, argument, blocker and continuity difficulties without automatic uploads.
