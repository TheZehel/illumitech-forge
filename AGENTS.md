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
Use MIT, preserve dependency notices. No Windows runtime support in 0.2.

Commits and releases use the configured human identity. Never attribute authorship or
coauthorship to Codex; do not add generated author trailers or agent branding to publication labels.
