# F068 plan

Extend scripts/documentation.py with four crate templates. Replace the CLI package's
obsolete candidate README with a concise product description, installation, diagnostics,
no-agent demo, workflow/skill links and explicit support limits. Update its Cargo description.
Keep library READMEs small and explain their role in the same release.

Run documentation coherence before source packaging. Compare each archive's README with
the generated source so a wrong archive fails before extraction/build. Regressions must
demonstrate a stale package README and mismatching archive bytes being rejected.

Use release/changes.json against 1.7.0 to verify target 1.7.1, then align workspace, exact
internal dependencies, lockfiles, UI/site/plugin metadata and current documentation. Preserve
historical release notes. Run repository gates and extracted-package checks. Prepare source,
binary, skill/plugin and preview artifacts outside the source worktree, with checksums and
an external release gate. Registry/GitHub readback follows separately authorized publication.
