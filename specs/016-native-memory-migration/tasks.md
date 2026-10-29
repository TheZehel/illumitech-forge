# Tarefas FORGE-016

Aceitação permanece pendente; conclusão exige os aceites do spec e evidência externa da revisão.

Candidato local: `scripts/memory_migration.py`, 16 testes offline em
`scripts/test_memory_migration.py` e `docs/native-memory-migration.md`. Inclui inventário
Docker sanitizado, backup de origem parada, restore novo, hashes/projeções, verificador MCP
e exportação de comandos de transição protegida por apply. Não executa comandos de produção.
O tooling genérico ainda requer ensaio integrado próprio. Separadamente, o operador validou
runtime nativo 3.2.9, restore isolado, MCP, continuidade Codex e cutover local;
ver `docs/expansion-runtime-acceptance.json`. Isso não homologa automaticamente o tooling.

- [ ] F016-T01: Plano e inventário sem mutação; evidência: F016-AC1/AC6; dry-run, checksum e preservação.
- [ ] F016-T02: Backup, restore isolado e verificação; evidência: F016-AC2/AC3; hashes, IDs, escopos e MCP em ambiente de ensaio. <!-- forge: {"dependencies":["F016-T01"]} -->
- [ ] F016-T03: Procedimentos protegidos de cutover/rollback; evidência: F016-AC4/AC5; testes de recusa e rollback em cópia. <!-- forge: {"dependencies":["F016-T02"]} -->
- [ ] F016-T04: Aceitação integrada do procedimento reutilizável em ambiente de ensaio; sessão nova, único escritor e restore do runtime local já foram comprovados pelo procedimento operacional separado. <!-- forge: {"dependencies":["F016-T03"]} -->
