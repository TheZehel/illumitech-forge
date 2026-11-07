# F066 implementation

Build from published v1.6.0 in an isolated worktree. Extend the existing Clap parser,
installation/workflow doctors, recovery journal and QA observations. Add typed diagnostic
and continuity helpers shared by CLI, terminal and Workbench. Keep schema 1 additive;
new command-catalog, diagnostic and continuity objects each declare schema 1. No DB changes.

Keep one source skill in `skills/illumitech-forge`; build allowlisted portable Agent Plugins
archives with host metadata, CLI compatibility and checksums. Preserve the internal core
skill and document installing exactly one public skill. Update current documentation through
its source templates; published installation references remain at the published version.

Run parser/contract, first-run, blockers and continuity tests, workspace format/Clippy/tests/docs,
frontend typecheck/build and available browser/host pilots. Version policy computes 1.7.0
from base 1.6.0; candidate version does not assert publication or operational acceptance.
