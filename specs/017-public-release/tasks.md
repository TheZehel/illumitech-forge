# Tarefas FORGE-017

Aceite externo pendente; conclusão exige os aceites do spec e evidência externa da revisão.
Implementação local em `scripts/distribution.py`, `scripts/public_readiness.py`, manifests
e publisher. Procedimentos e limites: `docs/public-distribution.md`. Checkboxes abaixo
permanecem dependentes da evidência externa, não da prosa do agente.

- [ ] F017-T01: Metadados e grafo de pacotes; evidência: F017-AC1; Cargo manifests e nomes coerentes.
- [ ] F017-T02: Verificação de empacotamento; evidência: F017-AC2/AC3/AC6; pacotes listados/extraídos e regressões negativas. <!-- forge: {"dependencies":["F017-T01"]} -->
- [ ] F017-T03: Auditoria e runbook público; evidência: F017-AC4; histórico e inventário sanitizados revisáveis. <!-- forge: {"dependencies":["F017-T02"]} -->
- [ ] F017-T04: Gate externo de publicação; evidência: F017-AC5; aprovação, registry e recibos reais, sem executar nesta tarefa. <!-- forge: {"dependencies":["F017-T03"]} -->
