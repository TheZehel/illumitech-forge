# Tarefas FORGE-031

Status: implementação e regressões locais disponíveis; marcas de aceite dependem do ledger externo da revisão.

- [ ] F031-T01: Definir contrato de achado e limites de rodadas de revisão. Aceites: FORGE-031. <!-- forge: {"dependencies": ["F021-T03", "F028-T03", "F030-T03"], "priority": 310} -->
- [ ] F031-T02: Integrar revisor nativo com permissões restritas e contexto mínimo suficiente. Aceites: FORGE-031. <!-- forge: {"dependencies": ["F031-T01"], "priority": 311} -->
- [ ] F031-T03: Validar falso handoff, achado sem suporte, alteração pelo revisor, desacordo e orçamento esgotado. Aceites: FORGE-031. <!-- forge: {"dependencies": ["F031-T02"], "priority": 312} -->

## Regressões mínimas previstas

- `reviewer_cannot_modify_implementation`
- `review_findings_reference_current_revision`
- `review_disagreement_does_not_fake_acceptance`

Nomes são contratos de cobertura a materializar em suites externas antes do marco;
não são testes existentes nem evidências de implementação.

## Cobertura local dos itens

T01: contratos públicos em `docs/m07-operating-contracts.md` e tipos estritos no domínio.
T02: scheduler/coordenador em `parallel.rs`, revisor nativo em `review.rs`, controles/UI.
T03: regressões reais de subprocessos, barreiras, sinais, conflitos, atribuição e limites
em `crates/forge-cli/tests/m07.rs`; fluxos de navegador em `apps/forge-ui/tests/m07.spec.ts`.
Checkboxes acima permanecem reservados ao aceite externo.
