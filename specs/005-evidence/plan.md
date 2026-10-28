# Plano FORGE-005

Implementar os contratos do spec sem alterar o Loop existente. Estado operacional em SQLite,
evidências vinculadas ao manifest e fingerprint do worktree. Falhas de contrato, memória
obrigatória ou permissão bloqueiam sem retry. Migrações usam cópia ensaiada e rollback separado.

## Verificação
- regressão de falso aceite textual.
- aprovação de revisão alterada rejeitada.

Registrar evidência em docs/verification.md; manter limitações explícitas.
