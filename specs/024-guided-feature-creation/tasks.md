# Tarefas FORGE-024

Status: backlog; marcas de conclusão dependem do ledger externo da revisão.

- [ ] F024-T01: Definir CLI e contratos de templates de projeto, preservando manifest v1. Aceites: FORGE-024. <!-- forge: {"dependencies": ["F021-T03"], "priority": 240} -->
- [ ] F024-T02: Implementar geração idempotente e preview; manter alterações locais e worktrees existentes. Aceites: FORGE-024. <!-- forge: {"dependencies": ["F024-T01"], "priority": 241} -->
- [ ] F024-T03: Validar projeto novo e existente, repetição de setup, conflitos, permissões e caminhos com espaços. Aceites: FORGE-024. <!-- forge: {"dependencies": ["F024-T02"], "priority": 242} -->

## Regressões mínimas previstas

- `feature_preview_does_not_modify_project`
- `feature_creation_is_idempotent_and_preserves_files`
- `feature_ids_and_dependencies_are_stable`

Nomes são contratos de cobertura a materializar em suites externas antes do marco;
não são testes existentes nem evidências de implementação.

## Cobertura implementada

As regressões nomeadas acima estão materializadas em `crates/forge-cli/src/` e
`apps/forge-ui/tests/workbench.spec.ts`. A execução local é cobertura de regressão;
checks de aceite, conta real e ledger continuam sob o coordenador externo.
