# illumitech-forge-cli

Part of [Illumitech Forge](https://github.com/TheZehel/illumitech-forge), an evidence-backed agent execution toolkit.

The binary is named `illumitech-forge`. After this version is published to the registry:

```sh
cargo install illumitech-forge-cli --version 0.3.0 --locked
illumitech-forge doctor
illumitech-forge release inspect release.yaml
illumitech-forge release run release.yaml --dry-run
```

Review the dry-run before execution. Tasks pass only through external gates, verified artifacts
or revision-bound human approval. Keep state and evaluators outside agent-writable repositories.
The CLI includes a read-only local dashboard, persistent queue controls and explicitly applied
per-user service definitions. It never publishes a release implicitly.

macOS ARM64 and Linux x86_64 are the primary platforms. Windows 11 x86_64 with PowerShell 7
is experimental: authenticated native acceptance and the latest service-host correction remain
unverified on Windows. Do not use its service integration for unattended workloads yet. Codex is the
validated client and Claude remains experimental. Install and authenticate clients separately.

ai-memory is optional. Only manifests selecting `memory.required: true` require its availability
and verified scope. Forge does not install memory hooks or bundle the ai-memory server.

See the repository's [quickstart](https://github.com/TheZehel/illumitech-forge/blob/main/docs/quickstart.md)
and [distribution gates](https://github.com/TheZehel/illumitech-forge/blob/main/docs/public-distribution.md).
MIT licensed. See LICENSE and THIRD_PARTY_NOTICES.
