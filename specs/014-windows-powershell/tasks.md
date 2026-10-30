# Tarefas FORGE-014

- [x] F014-T01 Inventariar dependências Unix e definir contratos portáveis; evidência: mapa de impactos e decisões.
- [x] F014-T02 Implementar processos, sinais e locks Windows; evidência: cancelamento de descendentes, timeout e crash nativos. <!-- forge: {"dependencies":["F014-T01"]} -->
- [x] F014-T03 Adaptar clientes, paths e exemplos PowerShell; evidência: espaços, Unicode, CRLF e instalação repetida. <!-- forge: {"dependencies":["F014-T01"]} -->
- [x] F014-T04 Adicionar CI Windows, ZIP e checksums; evidência: build/testes e instalação/preservação na CI; pacote baixado verificado (SHA256 externo e 396 hashes internos), sem executar/instalar. <!-- forge: {"dependencies":["F014-T02","F014-T03"]} -->
- [ ] F014-T05 Validar piloto real, regressões macOS/Linux e revisão humana; evidência: relatório de aceitação por plataforma. <!-- forge: {"dependencies":["F014-T04"]} -->

Reconciliação: [CI 40917514708 e limites](../../docs/expansion-evidence.md), revisão
`d015ff1a283b946e5f8b83fdb8e2c49e72fd9753`, e [inventário](../../docs/windows-portability.md).
T05 permanece aberto: piloto autenticado Windows 11/PowerShell 7, Ctrl+C interativo,
Git/gh reais e revisão humana não são substituídos pela CI ou preparação de fixture.
