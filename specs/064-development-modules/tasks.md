# Tarefas F064

- [x] F064-T01: catálogo, plano feature e plano QA sem execução; AC1–AC6. Validado pela suíte e gate externo F065.
- [x] F064-T08: priorizar execução de feature sobre o workflow existente, com validação por tarefa, antes do QA completo.
- [ ] F064-T02: descoberta de stack e adaptadores de verificações com cobertura explícita.
- [ ] F064-T03: relatório de achados, priorização e pesquisa com proveniência/data.
- [ ] F064-T04: loop corretivo limitado e checkpoints retomáveis, com testes de contexto esgotado e revisão alterada.
- [ ] F064-T05: módulo feature apoiado no gate QA.
- [ ] F064-T06: módulo refatoração com preservação de comportamento.
- [ ] F064-T07: módulo CI/CD com destino, artefato, aprovação, verificação e rollback.

T08: implementação local candidata disponível em `modules/feature.rs`, com comandos
prepare/start/status/resume/checkpoint e admissão de critérios/checks. T04 permanece
parcial: handoff pode ser extraído do journal; compactação/correção automáticas pendentes.

- [x] F064-T09: seletor interativo e instalação local. Implementação e operação local
  verificadas; ver validation.md. Feature/Conversa disponíveis, QA executável entregue em F065,
  Refatoração/CI/CD indisponíveis. Publicação e aceite global não fazem parte desta etapa.

## Issue tracking

- #7: QA adapters and prioritized findings (T02–T03).
- #8: bounded correction loop and durable context recovery (T04–T05).
- #9: refactoring module (T06).
- #10: CI/CD module (T07).
- #6: Linux native acceptance remains deferred.
