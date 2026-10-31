# Illumitech Forge workbench

React/TypeScript/Vite frontend for the same-origin Rust bridge. No browser access to
native credentials or arbitrary projects. Native configuration and managed authentication
stay in the installed Codex client; no paid API fallback exists.

```sh
npm ci --ignore-scripts
npm run typecheck
npm run build
cd ../..
cargo build -p illumitech-forge-cli
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
