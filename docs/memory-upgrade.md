# ai-memory 1.17.3 → 3.2.9

![Dois arquivos isolados conectados por luz laranja através de um prisma de verificação](assets/memory.webp)

The Forge integration uses MCP exclusively. These operator migration steps are separate from its
runtime and must never be executed by an agent tasked with modifying a project.

1. Inventory old binary/image digest, Docker/Colima profile, volume, config, client wrappers,
   hooks and MCP entries without exposing secret values. Preserve the old image.
2. Create an online snapshot via the server backup API. Briefly freeze the writer and take a
   complementary full data-directory backup. Store both outside the internal disk with restrictive
   permissions and SHA-256 checksums; preserve config, raw, wiki Git history, IDs and models.
3. Restore two copies to separate storage. Run old and target pinned images on an internal Docker
   network without production hooks, watchers, scheduled jobs, credentials or outbound LLM access.
4. Check database integrity offline, counts and ID digests; compare MCP page retrieval, search and
   explicit scope on both endpoints. Never open migrated data with the older binary.
5. Prove rollback using the untouched old backup. A successful startup alone is insufficient.
6. Only after the rehearsal passes, stop the active writer, obtain the final backup and migrate the
   updated corpus. Preserve the old container/volume for rollback; one writer per data directory.
7. Update the pinned client and native integrations together. Preserve unrelated settings and model
   selection. Review hook diffs for duplicate capture and run fresh client sessions. For the
   approved v0.1.0 scope, Codex is required and Claude remains experimental (ADR 0001).
8. On failure, stop the new writer and restore the final old-version backup using the old image.
   Do not point 1.17.3 at the migrated database.

Upstream reference: [v3.2.9](https://github.com/akitaonrails/ai-memory/releases/tag/v3.2.9),
[OKF migration](https://github.com/akitaonrails/ai-memory/blob/v3.2.9/docs/okf.md).
Operational backup paths and private inventories are deliberately kept outside this repository.
Actual outcomes and remaining gates appear in verification.md.

Validated result: [migration evidence](memory-upgrade-result.json). The current server is 3.2.9 on Docker/Colima. Native helper clients are used only for host hooks/bridges; no native server migration occurred.

The first real capture exposed an overlap between automatic first-project transcript backfill
and live hooks in 3.2.9. The native client's `config.toml` now has `backfill_on_start = false`.
Hook definitions and trust hashes were preserved. A subsequent session and an entirely new
disposable project each captured one start, prompt, stop and end, with no cross-project rows.
Historical records from the failed test were retained. Manual history import remains explicit.
