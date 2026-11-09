# Illumitech Forge

Use `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` and `cargo doc --workspace --no-deps` before delivery.

Specs and ADRs define intent, manifests execution contracts, SQLite operational state,
Git code and ai-memory contextual continuity. Never accept agent prose as evidence.
Keep evaluators and state outside agent-writable repositories. Never bypass client
permissions, overwrite unrelated files, select a different model, or publish implicitly.

Read the applicable `specs/*/{spec,plan,tasks}.md`. Query ai-memory through MCP using
workspace `illumitech` and this Git repo root. Durable memory writes require an explicit
user request. Read `skills/core/SKILL.md` for detailed execution procedures.

Keep source, examples and documentation free of credentials, personal paths and internal data.
Use MIT, preserve dependency notices. Windows remains experimental without a native pilot. Linux VM/container regressions do not prove native-account acceptance. Publication requires explicit authorization and evidence for the evaluated source; a version number is not an acceptance attestation.

Commits and releases use the configured human identity. Never attribute authorship or
coauthorship to Codex; do not add generated author trailers or agent branding to publication labels.

Before preparing any release, maintain and review `release/changes.json` against the last published version.
Use `release version --changes release/changes.json` to determine the highest compatibility impact,
and require its `--target VERSION` check as an external release gate. Follow `docs/release-version-policy.md`;
fix-only changes use patch, compatible features/deprecations minor, breaking contracts major.
Validate workspace/package/tag coherence separately; computed versions never authorize publication.

Keep every project-maintained README current with each delivery: root translations, crates and
apps. Update release templates and run `python3 scripts/documentation.py --write` followed by
its read-only check. Register new first-party READMEs in that inventory. Verify README bytes
inside the actual crate/archive and registry readback during publication. Preserve upstream
third-party texts and intentional historical release evidence.
