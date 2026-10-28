# Plano FORGE-009

Implementar os contratos do spec sem alterar o Loop existente. Estado operacional em SQLite,
evidências vinculadas ao manifest e fingerprint do worktree. Falhas de contrato, memória
obrigatória ou permissão bloqueiam sem retry. Migrações usam cópia ensaiada e rollback separado.

## Verificação
- import sem aceite herdado.
- Loop inspect e dry-run.

Registrar evidência em docs/verification.md; manter limitações explícitas.
