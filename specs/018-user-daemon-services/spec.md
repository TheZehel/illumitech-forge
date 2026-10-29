# FORGE-018 — Serviços daemon por usuário

## Limites e evidência
Especificação autorizada em 2026-10-29; implementação e aceitação desta frente pendentes.
Claude permanece experimental, explicitamente adiado: sem login ou validação nesta entrega.
ai-memory é opcional; somente `required: true` exige disponibilidade e escopo comprovados.
Estado, avaliadores e recibos operacionais ficam fora de repositórios graváveis pelo agente.
Prosa do agente, checkbox e telemetria não substituem gates externos vinculados à revisão.
Esta especificação não autoriza publicação, instalação global, ativação de serviço ou migração de produção.

## Dependências
FORGE-013; FORGE-015 precede esta frente no manifest. Windows depende de FORGE-014 e aceite nativo próprio.

## Intenção e contrato
Gerenciar execução supervisionada do daemon existente via launchd (macOS), systemd --user (Linux)
e Task Scheduler do usuário (Windows). Sem root/admin, serviço de sistema, shell interpolado ou WSL.
CLI `service plan|apply|status|start|stop|remove` recebe executável absoluto, releases-dir e state-dir
externos explícitos. `plan` só exibe configuração; `apply` instala desativado. Apenas `start` ativa.

## Aceites
- F018-AC1: arquivos gerados têm argv preservado para espaços/Unicode, identidade e hash de ownership;
  escapes específicos para plist, unit e XML. Rejeitar NUL/newline não representáveis e paths redirecionados.
- F018-AC2: apply/remove idempotentes apenas para recursos próprios intactos; recusar arquivo/tarefa
  modificados ou alheios. Remoção nunca apaga manifests, SQLite, logs históricos ou worktrees.
- F018-AC3: status distingue instalado/ativo/pausado/falhou/heartbeat antigo; não cria estado.
  start/stop usam supervisor nativo; locks do daemon impedem duas instâncias. Pausa/reconciliação
  permanecem contratos Forge, não são anulados por restart do serviço.
- F018-AC4: logs redigidos com rotação limitada (3 arquivos de até 1 MiB por stream), sem segredos
  em definição de serviço. Falha parcial reverte apenas recursos recém-criados e mantém recibo externo.
- F018-AC5: testes de fixtures/escaping/preservação sem ativação; smoke nativo por plataforma em
  ambiente descartável autorizado prova apply/start/status/stop/remove e crash/restart. Ausência de
  sessão systemd/launchd/Task Scheduler é gate pendente, não sucesso simulado.
