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
   selection. Review hook diffs for duplicate capture and run fresh Codex and Claude sessions.
8. On failure, stop the new writer and restore the final old-version backup using the old image.
   Do not point 1.17.3 at the migrated database.

Upstream reference: [v3.2.9](https://github.com/akitaonrails/ai-memory/releases/tag/v3.2.9),
[OKF migration](https://github.com/akitaonrails/ai-memory/blob/v3.2.9/docs/okf.md).
Operational backup paths and private inventories are deliberately kept outside this repository.
Actual outcomes and remaining gates appear in verification.md.
