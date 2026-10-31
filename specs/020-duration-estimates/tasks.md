# Tarefas FORGE-020

- [x] F020-T01: Contrato e cálculo puro de previsão; aceites AC1/AC2/AC6.
- [x] F020-T02: Medições persistentes e histórico comparável; aceites AC3/AC5. <!-- forge: {"dependencies":["F020-T01"]} -->
- [x] F020-T03: CLI, consultas e transições; aceites AC1/AC4/AC5/AC6. <!-- forge: {"dependencies":["F020-T02"]} -->
- [ ] F020-T04: Testes, documentação e aceite externo; aceites AC7. <!-- forge: {"dependencies":["F020-T03"]} -->

Checkboxes não substituem evidências externas. Não publicar nem executar Actions.

Validação externa de CLI: execução, relatório, ausência de escrita em consultas, espera humana,
timeout, cancelamento e reconciliação passaram. O gate completo de qualidade permanece no ledger
externo, separado da tarefa nativa original cujo log estruturado foi bloqueado após redação.
