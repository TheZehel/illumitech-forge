# F066 validation

Implementation branch: feat/agent-experience, based on published v1.6.0 / c5d80c5.
Version 1.7.0 evidence was collected before the publication revision. Runtime and skill
inputs are checked against that evidence during release preparation; publication-document
changes are validated separately. This file does not approve publication.

## Reproduction

On macOS use a canonical temporary directory and absolute Cargo target directory. These
avoid /var -> /private/var aliases and parent-traversal paths rejected by existing safety
checks. Local socket fixtures need loopback permission. The command-line-only SHA-256 optimization
keeps product source unchanged and avoids debug hashing overhead; a stack sample confirmed
sha2 compression dominated a read-only status query. The initial unoptimized serial run
was intentionally interrupted after passing unit, Feature and first-run groups. A parallel dashboard test was
unstable under build/test contention and passed in isolation; run the full suite serially:

```
TMPDIR=/private/tmp CARGO_TARGET_DIR=/absolute/target cargo test --workspace --locked --config 'profile.dev.package.sha2.opt-level=3' --config 'profile.test.package.sha2.opt-level=3' -- --test-threads=1
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
npm ci --prefix apps/forge-ui --ignore-scripts
npm run build --prefix apps/forge-ui
python3 scripts/test_package_agent_skill.py
python3 scripts/test_package.py
python3 scripts/test_documentation.py
python3 scripts/documentation.py
```

The browser fixture accepts FORGE_TEST_BINARY as an absolute path; use the release build
for distribution pilots. Debug builds are large and repeated pinned-evaluator hashing can
exceed the existing short browser query deadlines under contention. The browser suite
retains its original execution deadlines; do not turn a timeout into evidence of success.

## Observed pilots

- Standalone skill archive extracted into a fresh project's .agents/skills.
- Codex CLI 0.167.0 selector displayed Illumitech Forge. No model prompt was sent for discovery.
- A real native Codex Feature, using the candidate debug binary, passed its external
  greeting-content gate. An independent readback confirmed exact greeting bytes, unchanged
  skill files and the unrelated preserved file, and current evidence with completed=1.
- The no-agent demo passed inspect, dry-run, execution and report.
- Skill structural validation and deterministic packaging/checksum tests passed.
- UI typecheck/build and workspace documentation generation passed.
- All 307 Rust unit/integration tests passed, with zero ignored. The combined invocation
  then failed in documentation compilation because it retained an embedded asset path
  replaced during the final UI rebuild. Re-running all workspace doc tests with the final
  assets passed; no Rust unit/integration failure remained.
- All four release-binary Chrome scenarios passed: execution/reload/stale source; guided bug;
  missing-client explanation; and recovery from a failed state query.
- Final Clippy with warnings denied passed. Release artifacts and exact hashes are recorded
  in the external delivery report. VS Code discovery/terminal pilot is recorded below; Cursor remains unavailable.

## Host boundaries

Codex selector discovery and a native CLI workflow are observed separately. These do not
attest a public plugin-directory install. Cursor is unavailable on this machine.

The initial VS Code visual inspection was rejected by automatic approval review. The user
subsequently explicitly authorized an isolated visual pilot. A fresh temporary profile,
copied OpenAI extensions and a disposable Git project were used. The Codex selector showed
Illumitech Forge with its public description and Project scope. The integrated terminal
visibly returned `forge 1.7.0` from the extracted candidate. Skill files and preserved.txt
matched their pre-pilot bytes. No existing chat was opened and no model prompt was sent.

This validates standalone-skill discovery and CLI terminal access in VS Code. A full
model-driven IDE workflow and a plugin-directory installation were not tested; do not
claim those outcomes. The IDE uses the standalone skill, as the current official docs
state that the IDE extension does not support plugins (https://learn.chatgpt.com/docs/plugins).
Directory submissions and Cursor
acceptance remain separate stages. The remaining model-driven editor pilots are tracked
in [#12](https://github.com/TheZehel/illumitech-forge/issues/12).

## Post-publication VS Code Feature pilot

The published v1.7.0 macOS binary was operated through the Codex VS Code extension in the
disposable project. The first pass stopped on PATH attention. A reviewed process-only PATH
selected the verified binaries and existing native client without changing global settings.
The next pass found the fixture's missing HEAD; the operator initialized only that disposable
repository. This gap produced the F067 diagnosis patch.

Preview then passed. Apply initially stopped on the editor's external-directory permission.
Normal per-command approvals allowed the exact reviewed apply/start scope. The native
Codex Feature passed in one attempt, with initial and final external greeting checks.
Independent operator readback confirmed greeting.txt = Hello followed by one newline,
all eight preservation hashes, unchanged fixture HEAD and current passing Forge evidence.
Status/checkpoint confirmed completion without replay. This validates a bounded local
Feature through the IDE; it does not establish every module, platform or directory install.

A signed and notarized official Cursor 3.29.9 build was prepared in a temporary profile.
The user completed Cursor login and, after unreliable automated window control, the
extension's initial tour. The Codex selector displayed Illumitech Forge with Project scope.
The same bounded native Feature then passed in one attempt, including initial/final external
checks, exact greeting bytes, all eight preservation hashes, unchanged HEAD and current
passing status/checkpoint. The installed VS Code build was 1.109.2; native Codex CLI was
0.167.0. Active extension renderer builds were not independently fingerprinted, so the
receipts retain that version uncertainty. The native Cursor/Grok chat was not used.
No plugin-directory installation, other operating system or broader module acceptance is
inferred. Sanitized source/binary/gate bindings are in host-pilots.json.
