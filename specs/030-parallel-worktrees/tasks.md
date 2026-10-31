# Tarefas FORGE-030

Status: implementação e regressões locais disponíveis; marcas de aceite dependem do ledger externo da revisão.

- [ ] F030-T01: Definir scheduler, ownership e política de integração de resultados. Aceites: FORGE-030. <!-- forge: {"dependencies": ["F028-T03", "F029-T03"], "priority": 300} -->
- [ ] F030-T02: Implementar concorrência limitada e recuperação de processos/locks órfãos. Aceites: FORGE-030. <!-- forge: {"dependencies": ["F030-T01"], "priority": 301} -->
- [ ] F030-T03: Validar conflitos, falhas parciais, cancelamento, crash e equivalência de qualidade com execução serial. Aceites: FORGE-030. <!-- forge: {"dependencies": ["F030-T02"], "priority": 302} -->

## Regressões mínimas previstas

- `parallel_respects_dependencies_locks_and_budget`
- `parallel_cancel_ends_all_owned_processes`
- `integration_conflicts_preserve_unrelated_work`

Nomes são contratos de cobertura a materializar em suites externas antes do marco;
não são testes existentes nem evidências de implementação.

## Cobertura local dos itens

T01: contratos públicos em `docs/m07-operating-contracts.md` e tipos estritos no domínio.
T02: scheduler/coordenador em `parallel.rs`, revisor nativo em `review.rs`, controles/UI.
T03: regressões reais de subprocessos, barreiras, sinais, conflitos, atribuição e limites
em `crates/forge-cli/tests/m07.rs`; fluxos de navegador em `apps/forge-ui/tests/m07.spec.ts`.
Checkboxes acima permanecem reservados ao aceite externo.
