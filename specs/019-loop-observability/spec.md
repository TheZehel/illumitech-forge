# FORGE-019 — Paridade de observabilidade Loop

## Limites e evidência
Especificação autorizada em 2026-10-29; implementação local disponível; aceitação externa desta frente pendente.
Claude permanece experimental, explicitamente adiado: sem login ou validação nesta entrega.
ai-memory é opcional; somente `required: true` exige disponibilidade e escopo comprovados.
Estado, avaliadores e recibos operacionais ficam fora de repositórios graváveis pelo agente.
Prosa do agente, checkbox e telemetria não substituem gates externos vinculados à revisão.
Esta especificação não autoriza publicação, instalação global, ativação de serviço ou migração de produção.

## Dependências
FORGE-013; FORGE-018 precede esta frente no manifest. Não depende de memória, Claude ou credenciais para testes locais.

## Intenção e contrato
Instrumentar os seis spans inventariados em [loop-tracing-inventory.md](../../docs/loop-tracing-inventory.md).
Configuração proposta `automation.observability.langsmith: off|optional`, default `off`, compatível
com manifests antigos. `optional` também exige habilitação explícita e credencial no ambiente do operador;
variáveis herdadas sozinhas não autorizam rede. Dry-run/inspect não fazem chamadas de telemetria.

## Aceites
- F019-AC1: spans `forge.daemon_cycle`, `forge.release`, `forge.task`, `forge.validation_gates`,
  `forge.verification`, `forge.publish` mapeiam o inventário Loop e somente existem nas fases executadas.
  execution/run ID único, parent ID, release/task ID e attempt correlacionam eventos sem reaproveitar
  identidade entre tentativas. Publicação não executada não ganha span de sucesso.
- F019-AC2: allowlist de IDs, fase, attempt, dry_run, duração e resultado; valores redigidos/limitados
  a 256 bytes e metadata total a 4 KiB. Sem prompts, conteúdo, paths pessoais, stdout ou ambiente.
- F019-AC3: off/credencial ausente não enviam rede. Transporte bounded: fila até 128 spans, timeout
  de 2s e drain final de até 2s, sem retry ilimitado; saturação descarta telemetria com contador local.
  Erros em criação, entrada, saída, envio e flush não alteram resultado/gates, nem ocultam falha da tarefa.
- F019-AC4: HTTPS para provider; HTTP só em loopback de testes. Não seguir redirect com credenciais;
  sem fallback pago, outro modelo ou provider automático. Credenciais nunca persistem no manifest/SQLite.
- F019-AC5: servidor fake local comprova correlação, payload permitido, off sem rede, timeout, erro,
  cancelamento e falha de tarefa preservada. Aceite remoto exige credenciais/autorização separadas;
  SDK importado ou fake passando não comprova ingestão real LangSmith.
