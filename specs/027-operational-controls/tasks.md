# Tarefas FORGE-027

Status: backlog; marcas de conclusão dependem do ledger externo da revisão.

- [ ] F027-T01: Expor API local tipada e segura de controle; manter dashboard legado somente leitura. Aceites: FORGE-027. <!-- forge: {"dependencies": ["F023-T03", "F025-T03"], "priority": 270} -->
- [ ] F027-T02: Implementar fluxos de aprovação e execução com journal e reconciliação. Aceites: FORGE-027. <!-- forge: {"dependencies": ["F027-T01"], "priority": 271} -->
- [ ] F027-T03: Validar concorrência, pedidos duplicados, origem indevida, cancelamento e revisão alterada. Aceites: FORGE-027. <!-- forge: {"dependencies": ["F027-T02"], "priority": 272} -->

## Regressões mínimas previstas

- `ui_controls_reuse_engine_authority`
- `ui_stale_revision_cannot_be_approved`
- `ui_duplicate_actions_are_idempotent`

Nomes são contratos de cobertura a materializar em suites externas antes do marco;
não são testes existentes nem evidências de implementação.

## Local implementation

Implementation: `operational.rs`, authenticated workbench transport, engine controls and the UI evidence panel. Public contracts and compatibility/limitations are documented in
[the M06 operating contracts](../../docs/m06-operating-contracts.md). Local regressions live in
`crates/forge-cli/tests/m06.rs`, the corresponding Rust unit modules and
`apps/forge-ui/tests/m06.spec.ts`. Checkboxes remain owned by the external acceptance ledger;
local validation and protocol fixtures do not establish native platform or remote acceptance.
