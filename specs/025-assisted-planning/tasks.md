# Tarefas FORGE-025

Status: backlog; marcas de conclusão dependem do ledger externo da revisão.

- [ ] F025-T01: Implementar contrato de proposta e validação determinística, com timeout e limites. Aceites: FORGE-025. <!-- forge: {"dependencies": ["F022-T03", "F024-T03"], "priority": 250} -->
- [ ] F025-T02: Integrar cliente nativo e estimador existente; não delegar autoridade a conteúdo de repositório ou memória. Aceites: FORGE-025. <!-- forge: {"dependencies": ["F025-T01"], "priority": 251} -->
- [ ] F025-T03: Testar plano inválido, ambiguidade relevante, ciclo, referência inexistente e revisão do plano antes de executar. Aceites: FORGE-025. <!-- forge: {"dependencies": ["F025-T02"], "priority": 252} -->

## Regressões mínimas previstas

- `planning_rejects_invalid_dags_and_acceptance`
- `planning_estimates_before_execution`
- `planning_never_grants_publish_authority`

Nomes são contratos de cobertura a materializar em suites externas antes do marco;
não são testes existentes nem evidências de implementação.

## Cobertura implementada

As regressões nomeadas acima estão materializadas em `crates/forge-cli/src/` e
`apps/forge-ui/tests/workbench.spec.ts`. A execução local é cobertura de regressão;
checks de aceite, conta real e ledger continuam sob o coordenador externo.
