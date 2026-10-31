# Tarefas FORGE-023

Status: backlog; marcas de conclusão dependem do ledger externo da revisão.

- [ ] F023-T01: Validar protocolo por spike e fixtures; registrar ADR de compatibilidade sem pedir escolha técnica rotineira ao usuário. Aceites: FORGE-023. <!-- forge: {"dependencies": ["F021-T03", "F022-T03"], "priority": 230} -->
- [ ] F023-T02: Implementar MVP web local com React/TypeScript/Vite e ponte Rust em origem local autenticada; manter empacotamento desktop para 0.9. Aceites: FORGE-023. <!-- forge: {"dependencies": ["F023-T01"], "priority": 231} -->
- [ ] F023-T03: Validar fluxos com browser real e Codex autenticado; falhas, recusa e reconexão não duplicam ações. Aceites: FORGE-023. <!-- forge: {"dependencies": ["F023-T02"], "priority": 232} -->

## Regressões mínimas previstas

- `workbench_session_and_project_isolation`
- `workbench_approval_denial_is_preserved`
- `workbench_reconnect_does_not_repeat_writes`

Nomes são contratos de cobertura a materializar em suites externas antes do marco;
não são testes existentes nem evidências de implementação.

## Cobertura implementada

As regressões nomeadas acima estão materializadas em `crates/forge-cli/src/` e
`apps/forge-ui/tests/workbench.spec.ts`. A execução local é cobertura de regressão;
checks de aceite, conta real e ledger continuam sob o coordenador externo.
