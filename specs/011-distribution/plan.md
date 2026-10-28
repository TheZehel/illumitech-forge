# Plano FORGE-011

Implementar os contratos do spec sem alterar o Loop existente. Estado operacional em SQLite,
evidências vinculadas ao manifest e fingerprint do worktree. Falhas de contrato, memória
obrigatória ou permissão bloqueiam sem retry. Migrações usam cópia ensaiada e rollback separado.

## Verificação
- CI nas duas plataformas.
- binários verificados e release privada.

Registrar evidência em docs/verification.md; manter limitações explícitas.
