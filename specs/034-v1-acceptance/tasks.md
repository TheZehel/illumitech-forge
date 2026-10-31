# Tarefas FORGE-034

Status: estabilização local em andamento; marcas de conclusão dependem do ledger externo da revisão.

- [ ] F034-T01: Congelar interfaces da candidata, inventariar limites e completar documentação e migrações. Aceites: FORGE-034. <!-- forge: {"dependencies": ["F021-T03", "F022-T03", "F023-T03", "F024-T03", "F025-T03", "F026-T03", "F027-T03", "F028-T03", "F029-T03", "F030-T03", "F031-T03", "F032-T03", "F033-T03"], "priority": 340} -->
- [ ] F034-T02: Executar aceitação real e testes de recuperação, isolamento, consumo e duração; produzir relatório verificável. Aceites: FORGE-034. <!-- forge: {"dependencies": ["F034-T01"], "priority": 341} -->
- [ ] F034-T03: Preparar notas, pacotes e hashes para aprovação final da 1.0.0; não publicar neste programa de implementação. Aceites: FORGE-034. <!-- forge: {"dependencies": ["F034-T02"], "priority": 342} -->

## Regressões mínimas previstas

- `v1_end_to_end_with_external_gates`
- `v1_crash_upgrade_and_project_isolation`
- `v1_distribution_inventory_and_support_matrix`

Nomes são contratos de cobertura a materializar em suites externas antes do marco;
não são testes existentes nem evidências de implementação.
