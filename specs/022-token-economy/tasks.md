# Tarefas FORGE-022

Status: backlog; marcas de conclusão dependem do ledger externo da revisão.

- [ ] F022-T01: Implementar coleta local e baseline reproduzível; registrar limitações de cada cliente. Aceites: FORGE-022. <!-- forge: {"dependencies": ["F021-T03"], "priority": 220} -->
- [ ] F022-T02: Implementar contexto limitado e perfis explícitos, sem novos serviços pagos ou hooks. Aceites: FORGE-022. <!-- forge: {"dependencies": ["F022-T01"], "priority": 221} -->
- [ ] F022-T03: Comparar baseline e otimização em fixtures equivalentes com gates externos; registrar resultados e qualidade. Aceites: FORGE-022. <!-- forge: {"dependencies": ["F022-T02"], "priority": 222} -->

## Regressões mínimas previstas

- `usage_updates_are_not_double_counted`
- `context_preserves_instructions_and_project_isolation`
- `economy_counts_failed_and_planning_calls`

Nomes são contratos de cobertura a materializar em suites externas antes do marco;
não são testes existentes nem evidências de implementação.
