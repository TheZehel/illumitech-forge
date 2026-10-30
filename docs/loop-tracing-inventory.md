# Inventário Loop → Forge (FORGE-019)

Inspeção somente leitura em 2026-10-29, Loop commit
`8014289baf815750ab176453d6c61eef5fb4ffdb`; os três módulos abaixo não tinham diff local.
`loop_engineering/core/tracing.py` SHA256:
`814544892d06cc575c11670cf9ed114859f31133b565057d2a2c63f41823def6`.
Este inventário comprova leitura de código; não comprova telemetria entregue ao provider.

## Comportamento observado

`TraceSink.span` retorna `nullcontext`. `trace_sink_for_mode` só seleciona
`LangSmithTraceSink` para a string exata `optional`; outros valores retornam noop.
O sink LangSmith exige valores não vazios de `LANGSMITH_TRACING` e `LANGSMITH_API_KEY`.
Isso usa truthiness, não parsing booleano: a string `false` também é não vazia.
Importa `langsmith.trace` dinamicamente; exceção de import desabilita o sink.
`span` chama `trace(name=name, metadata=metadata or {})`; falha nessa chamada retorna noop.
Não há redaction, limite de metadata, transporte, timeout ou correlação própria nesse módulo.
Falhas de entrada/saída do context manager retornado não estão cobertas pelo try de `span`.
Forge deverá garantir fail-open em todo o ciclo, sem engolir erros da operação instrumentada.

| Origem | Span Loop | Metadata explícita | Equivalente previsto Forge |
| --- | --- | --- | --- |
| core/daemon.py | loop.daemon_cycle | dry_run | forge.daemon_cycle |
| core/daemon.py | loop.release | release_id, dry_run | forge.release |
| core/orchestrator.py | loop.task | release_id, task_id, dry_run | forge.task |
| core/orchestrator.py | loop.validation_gates | release_id, task_id, dry_run | forge.validation_gates |
| core/orchestrator.py | loop.verification | release_id, task_id, attempt | forge.verification |
| core/daemon.py | loop.publish | release_id | forge.publish |

O daemon injeta o sink configurado no orquestrador. Seu loop contínuo pede sink `optional`
para `loop.daemon_cycle`; a construção padrão do orquestrador usa noop. No daemon,
release engloba execução e publicação; task, validation_gates e verification são fases irmãs.
Publish só é alcançado após execução bem-sucedida e fora de dry-run; o span pode envolver
uma decisão de não publicar, portanto sua existência não é recibo de publicação.
`tests/test_tracing.py` prova apenas noop de optional sem as variáveis de ambiente.
Não há prova nesse teste de transporte remoto, redaction, nesting ou fail-open no exit.

## Critério de paridade

Cobrir as seis fases com IDs e resultado consistente, testes fake e metadata permitida.
Nomes `forge.*` identificam o novo runtime. A ausência de rede por padrão e no dry-run,
limites/redaction e contenção de falhas são requisitos Forge adicionais, não capacidades
atribuídas ao Loop. O contrato completo está em
[FORGE-019](../specs/019-loop-observability/spec.md).
Aceite remoto LangSmith permanece pendente de credenciais e autorização próprias.

Implementação e limites comprovados localmente: [observability.md](observability.md).
