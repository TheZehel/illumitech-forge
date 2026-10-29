# Tarefas FORGE-014

- [ ] F014-T01 Inventariar dependências Unix e definir contratos portáveis; evidência: mapa de impactos e decisões.
- [ ] F014-T02 Implementar processos, sinais e locks Windows; evidência: cancelamento de descendentes, timeout e crash nativos. <!-- forge: {"dependencies":["F014-T01"]} -->
- [ ] F014-T03 Adaptar clientes, paths e exemplos PowerShell; evidência: espaços, Unicode, CRLF e instalação repetida. <!-- forge: {"dependencies":["F014-T01"]} -->
- [ ] F014-T04 Adicionar CI Windows, ZIP e checksums; evidência: build/testes e pacote baixado verificado. <!-- forge: {"dependencies":["F014-T02","F014-T03"]} -->
- [ ] F014-T05 Validar piloto real, regressões macOS/Linux e revisão humana; evidência: relatório de aceitação por plataforma. <!-- forge: {"dependencies":["F014-T04"]} -->
