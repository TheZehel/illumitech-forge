# Illumitech Forge 1.7.1

This patch corrects the crates.io product description and installation documentation,
and explains a missing initial Git revision before Feature preparation.

- The CLI crate README now describes first-run diagnosis, Feature/QA, contracts, continuity,
  terminal/Workbench access and the portable skill. Its installation command matches 1.7.1.
- All four package READMEs are generated from the current release metadata. Distribution
  rejects stale source documentation and verifies the actual README bytes in each archive.
- Workflow doctor reports a separate Git revision prerequisite. An unborn repository is
  blocked with a clear request to review an initial commit; a failed probe stays unverified.
  Diagnosis remains read-only and creates no commit, inference or execution state.
- Skill setup and development guidance explain that prerequisite and reviewed executable
  selection when multiple Forge installations exist.

Existing state and manifest contracts remain schema 1; no dependency or telemetry is added.
The already published 1.7.0 tag and artifacts remain unchanged.

Bounded post-publication pilots on macOS ARM64 observed project-skill discovery and one
model-driven Feature workflow through the Codex extension in VS Code and Cursor, with
external gates, exact output bytes, preservation and continuity readback. Those pilots used
the published 1.7.0 binary and do not establish marketplace/directory installation, other
operating systems or all workflows. Codex remains the native executor. Native Linux
acceptance is deferred; Windows and Claude remain experimental.

Release artifacts include macOS ARM64 binaries, four source crates, source and local site
preview archives, standalone skill/plugin candidates, verification metadata and checksums.
Website deployment, marketplace submissions and global CLI upgrades are separate operations.
