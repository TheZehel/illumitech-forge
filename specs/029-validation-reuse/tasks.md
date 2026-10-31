# Tarefas FORGE-029

Status: backlog; marcas de conclusão dependem do ledger externo da revisão.

- [ ] F029-T01: Definir chaves, níveis de confiança e retenção de evidências. Aceites: FORGE-029. <!-- forge: {"dependencies": ["F021-T03", "F028-T03"], "priority": 290} -->
- [ ] F029-T02: Implementar cache local verificável sem pular gates obrigatórios ou perder journals. Aceites: FORGE-029. <!-- forge: {"dependencies": ["F029-T01"], "priority": 291} -->
- [ ] F029-T03: Validar mudanças em código, avaliador, manifest, permissões, toolchain, cache corrompido e artefatos ignorados. Aceites: FORGE-029. <!-- forge: {"dependencies": ["F029-T02"], "priority": 292} -->

## Regressões mínimas previstas

- `cache_invalidates_changed_inputs_and_evaluators`
- `cache_never_transfers_human_approval`
- `cache_reports_provenance_and_corruption`

Nomes são contratos de cobertura a materializar em suites externas antes do marco;
não são testes existentes nem evidências de implementação.

## Local implementation

Implementation: GateCache, `validation_cache.rs`, protected metadata and gate evidence journals. Public contracts and compatibility/limitations are documented in
[the M06 operating contracts](../../docs/m06-operating-contracts.md). Local regressions live in
`crates/forge-cli/tests/m06.rs`, the corresponding Rust unit modules and
`apps/forge-ui/tests/m06.spec.ts`. Checkboxes remain owned by the external acceptance ledger;
local validation and protocol fixtures do not establish native platform or remote acceptance.
