# FORGE-040 — Primeira experiência e diagnóstico

Revisão após a publicação da 1.1.0: reduzir a dependência de um checkout e de Python
para experimentar o motor; explicar versões concorrentes sem executar binários descobertos;
atualizar a apresentação e entregar as correções na versão 1.1.1.

## Aceites

- `demo create DIRECTORY` prepara um exemplo novo e descartável com Git, comando nativo,
  manifest e avaliador externo fixado por hash. Não requer cliente, conta, Python ou memória.
- Criar não executa a release. A saída fornece argv para inspect, dry-run, run e report,
  todos com o mesmo estado externo. O exemplo verifica um artefato e preserva outro arquivo.
- Diretórios existentes, symlinks e arquivos preexistentes são preservados. Falhas deixam
  o diretório recém-criado para diagnóstico, sem limpeza destrutiva automática.
- O diagnóstico inspeciona ambos os nomes, ordem efetiva de PATH e seleção diferente do
  executável em execução. Não executa candidatos, não inventa suas versões e não altera PATH.
- `init` não cria roteamento de memória por padrão. `--memory-workspace NAME` seleciona
  explicitamente o workspace; configurações existentes nunca são substituídas.
- `status`/`report` usam `effective_status` para o exit code: evidências antigas após mudança
  de fonte retornam 2, preservando o snapshot histórico e o diagnóstico JSON somente leitura.
- README PT/EN e início rápido indicam os comandos disponíveis em 1.1.1 e os limites de suporte. GitHub permanece privado; memória é opcional.
- Gates externos verificam o ciclo completo, recusa de sobrescrita, integridade do avaliador,
  ausência de inferência e diagnóstico de candidatos desconhecidos sem executá-los.

A publicação 1.1.1 foi autorizada após a revisão local, e é rastreada por FORGE-041.
Mudança de visibilidade, instalação global, serviços e novos Actions não fazem parte da entrega. Windows/Claude continuam experimentais; não há novo aceite nativo Linux.
