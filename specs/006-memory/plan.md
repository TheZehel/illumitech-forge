# Plano FORGE-006

Implementar os contratos do spec sem alterar o Loop existente. Estado operacional em SQLite,
evidências vinculadas ao manifest e fingerprint do worktree. Falhas de contrato, memória
obrigatória ou permissão bloqueiam sem retry. Migrações usam cópia ensaiada e rollback separado.

## Verificação
- nova sessão.
- isolamento de dois projetos e sem hooks duplicados.

Registrar evidência em docs/verification.md; manter limitações explícitas.
