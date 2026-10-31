# Tarefas FORGE-032

Status: backlog; marcas de conclusão dependem do ledger externo da revisão.

- [ ] F032-T01: Definir adaptador GitHub opt-in e mapeamento rastreável. Aceites: FORGE-032. <!-- forge: {"dependencies": ["F025-T03", "F026-T03", "F027-T03", "F031-T03"], "priority": 320} -->
- [ ] F032-T02: Integrar importação e draft PR com journals e políticas existentes. Aceites: FORGE-032. <!-- forge: {"dependencies": ["F032-T01"], "priority": 321} -->
- [ ] F032-T03: Executar piloto em repo descartável autorizado e fixtures para erro/retry/remoto alterado; sem Actions implícita. Aceites: FORGE-032. <!-- forge: {"dependencies": ["F032-T02"], "priority": 322} -->

## Regressões mínimas previstas

- `issue_content_does_not_grant_execution_authority`
- `draft_retry_reconciles_remote_state`
- `github_mutations_require_declared_scope`

Nomes são contratos de cobertura a materializar em suites externas antes do marco;
não são testes existentes nem evidências de implementação.

## Cobertura local implementada (não altera o ledger externo)

- F032-T01: CLI de importação e `Manifest.source`; identidade/hash, offline/live, preview,
  apply create-only e rejeição de demanda inválida têm regressões locais.
- F032-T02: Engine/relatórios/UI e publisher existente integram fonte/revisão/evidências,
  política Actions e journal de criação/readback sem duplicação.
- F032-T03: Fixtures locais cobrem timeout de provedor após criação e divergência remota;
  testes de navegador usam a UI empacotada. Piloto remoto real pendente de autorização separada.

As caixas permanecem sob autoridade do ledger externo do coordenador.
