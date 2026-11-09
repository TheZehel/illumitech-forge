# F070 plan

Reuse the deterministic standalone ZIP and its existing resources, with a self-contained
bounded installer shipped in the binary archive. Validate all content/compatibility before
creating directories, stage only this installer-owned output and preserve existing targets.
Return concise JSON preview/apply receipts and actionable failures, without runtime secrets.

Keep skill entrypoint short; put installation detail in setup and a public getting-started
guide. Include reviewed pinned skills CLI examples with --skill illumitech-forge and an
explicit agent/project scope; do not run an unfiltered all-skills/global install. The CLI
is separate software with its own behavior; a catalog install is not a Forge executor.

Use independent installation checks and source regression tests. A disposable public 1.7.1
install can establish the catalog route, while local candidate ZIP tests establish the new
installer. Label versions and evidence separately. First-use/native editor pilots, source
publication, website deployment and marketplace submission require their own evidence.
