# F070 — Verified project skill installation

Tracks [issue #16](https://github.com/TheZehel/illumitech-forge/issues/16).

The public skill and 1.7.1 archives exist. Operator-captured evidence now covers pinned
public v1.7.1 catalog file installation and Codex CLI first-use; the new installer remains
a local candidate. Keep CLI and skill usable independently.

Acceptance:
- Provide a stdlib-only `scripts/install_agent_skill.py` for the standalone release ZIP,
  with required `--archive`, `--sha256`, `--root`, `--host codex|cursor`, `--forge-binary`.
  Default is preview/read-only; `--apply` writes exactly one project skill installation.
- Verify the caller-provided whole-archive hash and complete inner SHA256SUMS, safe bounded
  ZIP members, public skill identity and required resources before any target writes.
  Reject plugin/wrong-skill archives, traversal, symlinks, duplicates and corrupt/extra files.
- Probe only the explicitly selected executable with bounded `--version`/JSON contracts.
  Require Forge >=1.7.0,<2.0.0 and command_catalog/continuity schema 1. Missing/incompatible
  CLI is actionable setup failure and leaves the project unchanged; never run PATH candidates.
- Require an existing selected Git project root. Install under .agents/skills/illumitech-forge
  for Codex CLI/the Codex extension (including VS Code and Cursor), or
  .cursor/skills/illumitech-forge for the native Cursor Agent loader. Host selects the loader,
  not the editor. Preserve every existing install
  and unrelated file; reject symlink ancestors, no overwrite/force, no user/global config,
  authentication, PATH, hooks, background service or native inference.
- Include the installer in binary archives. Document release ZIP and source/catalog routes,
  choosing only illumitech-forge (not internal forge-workflow), pinned releases, first-run
  diagnosis, one small Feature and sanitized voluntary feedback with version/host/module.
- Prove preview, apply, corruption/incompatibility rejection and preservation in disposable
  Codex/Cursor project layouts. Separately verify a pinned public repository installation
  through skills CLI before advertising that route as observed. Directory submission and
  editor model execution remain distinct from file installation tests.

Observed public baseline: skills CLI 1.7.6 installed public v1.7.1 at
`0391aeb275f1e0d2e18842cff1e26c31000513e5`, selecting only `illumitech-forge` and one project
agent. Both targeted agents used `.agents/skills/illumitech-forge`. Public-release doctor,
no-agent demo and a Codex CLI greeting Feature passed the operator's independent checks.
This does not establish new editor/model or marketplace acceptance, or publish the new installer.

No new public release, website deployment, directory submission or global upgrade is implied.
