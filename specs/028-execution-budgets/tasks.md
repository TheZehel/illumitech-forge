# Tarefas FORGE-028

Status: backlog; marcas de conclusão dependem do ledger externo da revisão.

- [ ] F028-T01: Definir política local e contrato de orçamento com dados ausentes explícitos. Aceites: FORGE-028. <!-- forge: {"dependencies": ["F022-T03", "F025-T03"], "priority": 280} -->
- [ ] F028-T02: Integrar orçamento à seleção de tarefas, clientes e UI sem alterar modelos silenciosamente. Aceites: FORGE-028. <!-- forge: {"dependencies": ["F028-T01"], "priority": 281} -->
- [ ] F028-T03: Validar orçamento esgotado, saldo inacessível, recuperação e ausência de disparos implícitos de CI. Aceites: FORGE-028. <!-- forge: {"dependencies": ["F028-T02"], "priority": 282} -->

## Regressões mínimas previstas

- `budget_unknown_is_not_zero_or_unlimited`
- `budget_stops_before_new_client_calls`
- `ci_requires_explicit_budgeted_activation`

Nomes são contratos de cobertura a materializar em suites externas antes do marco;
não são testes existentes nem evidências de implementação.

## Local implementation

Implementation: manifest/task Budget, `budget.rs`, durable native reservations and engine admission. Public contracts and compatibility/limitations are documented in
[the M06 operating contracts](../../docs/m06-operating-contracts.md). Local regressions live in
`crates/forge-cli/tests/m06.rs`, the corresponding Rust unit modules and
`apps/forge-ui/tests/m06.spec.ts`. Checkboxes remain owned by the external acceptance ledger;
local validation and protocol fixtures do not establish native platform or remote acceptance.
