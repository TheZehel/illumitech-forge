# Tarefas FORGE-026

Status: backlog; marcas de conclusão dependem do ledger externo da revisão.

- [ ] F026-T01: Definir catálogo e seleção de receitas sem instalação implícita. Aceites: FORGE-026. <!-- forge: {"dependencies": ["F024-T03", "F025-T03"], "priority": 260} -->
- [ ] F026-T02: Implementar renderização/upgrade com preview, merge conservador e proveniência. Aceites: FORGE-026. <!-- forge: {"dependencies": ["F026-T01"], "priority": 261} -->
- [ ] F026-T03: Validar todas as receitas em projetos descartáveis, setup repetido e conflitos. Aceites: FORGE-026. <!-- forge: {"dependencies": ["F026-T02"], "priority": 262} -->

## Regressões mínimas previstas

- `template_origin_and_revision_are_pinned`
- `template_update_preserves_customizations`
- `template_never_executes_remote_instructions`

Nomes são contratos de cobertura a materializar em suites externas antes do marco;
não são testes existentes nem evidências de implementação.

## Local implementation

Implementation: `templates.rs`, guided creation and catalog Recipe data. Public contracts and compatibility/limitations are documented in
[the M06 operating contracts](../../docs/m06-operating-contracts.md). Local regressions live in
`crates/forge-cli/tests/m06.rs`, the corresponding Rust unit modules and
`apps/forge-ui/tests/m06.spec.ts`. Checkboxes remain owned by the external acceptance ledger;
local validation and protocol fixtures do not establish native platform or remote acceptance.
