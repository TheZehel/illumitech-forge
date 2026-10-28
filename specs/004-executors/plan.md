# Plano FORGE-004

Implementar os contratos do spec sem alterar o Loop existente. Estado operacional em SQLite,
evidências vinculadas ao manifest e fingerprint do worktree. Falhas de contrato, memória
obrigatória ou permissão bloqueiam sem retry. Migrações usam cópia ensaiada e rollback separado.

## Verificação
- processos órfãos e timeout.
- Codex real e subprocessos; Claude experimental nesta entrega (ADR 0001).

Registrar evidência em docs/verification.md; manter limitações explícitas.
