# Illumitech Forge v0.3.0 — candidata

Esta revisão amplia o Forge com painel local, serviços por usuário, observabilidade opcional
e pacotes preparados para crates.io. Abertura pública e publicação são operações explícitas,
condicionadas ao aceite da revisão avaliada. Claude continua experimental e adiado.

## Mudanças

- `dashboard`: consulta local somente leitura de releases, tarefas, gates, eventos e heartbeat,
  com respostas limitadas, escape HTML e validação de Host/Origin.
- `service plan|apply|status|start|stop|remove`: launchd, systemd do usuário e Task Scheduler,
  preservação de recursos alheios, logs limitados e recibos de operação. `--definition-dir`
  permite manter o estado em SSD externo sem instalar um plist em volume `noowners`.
- Telemetria LangSmith opcional, desligada por padrão, sem prompts ou saída de processos.
  Transporte limitado e falhas de telemetria não alteram o resultado dos gates.
- Quatro pacotes `illumitech-forge-*`, com dependências internas versionadas, inventários,
  MIT e avisos de terceiros. O binário continua `illumitech-forge`.
- `release publish --expected-visibility PRIVATE|PUBLIC`: verifica a visibilidade existente;
  não altera o repositório nem contorna aprovação, revisão, tag ou checksums.
- Ferramentas de ensaio da migração ai-memory, com backup de origem parada, restore isolado,
  verificação MCP e procedimentos de transição. Memória continua opcional para usar o Forge.

## Evidências

Os gates locais passaram: fmt, Clippy sem warnings, 78 testes Rust, documentação, 16 testes
de migração, cinco de distribuição e um de embalagem. Os quatro arquivos Cargo foram extraídos
e compilados em workspace temporário; isso não substitui resolução e upload reais no registry.
OSV não retornou advisories para as 222 dependências consultadas em 2026-10-29.

O [recibo operacional](expansion-runtime-acceptance.json) registra Codex real com implementação
e revisão somente leitura, gates externos, preservação de arquivos, painel no Chrome e ciclo
launchd com crash/restart e pausa persistente. O runtime local ai-memory 3.2.9 foi migrado para
launchd; MCP e captura de sessão nova passaram com a VM Colima dedicada parada. Backups e
estado operacional privado permanecem fora do repositório.

Antes de publicar: CI macOS/Linux/Windows da revisão exata, revisão dos artefatos e checksums,
autenticação/ownership no crates.io e aceite humano vinculado ao fingerprint.

## Limites

- Windows 11 x86_64/PowerShell 7 permanece candidato até o piloto autenticado adiado pelo usuário.
- O ciclo nativo de serviços em Linux/Windows e a ingestão real LangSmith têm gates próprios pendentes.
- O tooling genérico de migração tem testes offline e contratos MCP conferidos; sua execução integral
  de ensaio é distinta da migração operacional já realizada.
- Claude real não foi executado. Não há fallback silencioso para outro modelo ou API paga.
- O Loop Python continua disponível; nenhuma fila de projetos foi migrada automaticamente.

Consulte [distribuição](public-distribution.md), [serviços](user-services.md),
[dashboard](dashboard.md), [observabilidade](observability.md) e
[migração nativa](native-memory-migration.md).
