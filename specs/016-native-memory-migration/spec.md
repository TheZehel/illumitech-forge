# FORGE-016 — Migração nativa do ai-memory

## Limites e evidência
Especificação autorizada em 2026-10-29; implementação e aceitação desta frente pendentes.
Claude permanece experimental, explicitamente adiado: sem login ou validação nesta entrega.
ai-memory é opcional; somente `required: true` exige disponibilidade e escopo comprovados.
Estado, avaliadores e recibos operacionais ficam fora de repositórios graváveis pelo agente.
Prosa do agente, checkbox e telemetria não substituem gates externos vinculados à revisão.
Esta especificação não autoriza publicação, instalação global, ativação de serviço ou migração de produção.

## Dependências
FORGE-006 e FORGE-010; FORGE-017 para empacotamento revisável das ferramentas. Não depende de aceite Claude.

## Intenção e contrato
Preparar ferramenta/runbook de ensaio Docker → binário nativo; não implementar outro servidor de memória.
Plano declarativo exige versão e SHA256 do binário, origem inventariada, destino de backup externo,
diretório nativo novo e endpoint loopback separado. Preview é padrão; toda mutação exige `--apply`
e fase explícita (`rehearse`, `cutover` ou `rollback`). Nunca inferir produção por diretório atual.
Forge continua consultando memória exclusivamente via MCP; inspeção offline é só de cópias isoladas.

## Aceites
- F016-AC1: inventariar versão, volumes, schema, configurações e escritores sem exibir credenciais;
  validar checksum/versão antes de executar binário. Recusar destino existente, symlink ou sobreposição.
- F016-AC2: backup consistente inclui banco/WAL quando aplicável, corpus, configuração e manifest de
  hashes; quiescência ou API de backup suportada, nunca simples cópia de DB vivo. Restauração comprovada
  antes de cutover, backup externo ao volume de origem e ao worktree.
- F016-AC3: ensaio restaura em diretório novo e endpoint exclusivo, sem schedulers, LLM externo,
  hooks de produção ou escritor concorrente. Validar MCP initialize/consulta real, integridade,
  IDs, contagens, conteúdo e isolamento de dois escopos; HTTP 200 não é aceite MCP.
- F016-AC4: cutover exige autorização operacional separada, parar escritas, backup final, restauração
  validada, único escritor e continuidade Codex em sessão nova. Não duplicar hooks nem mudar modelo.
- F016-AC5: rollback para binário antigo usa cópia compatível preservada; nunca abrir schema migrado
  com versão antiga. Se houve novas escritas, bloquear rollback automático até reconciliar divergência.
- F016-AC6: testes offline sem Docker cobrem preview sem escrita, checksum inválido, destino ocupado,
  writer ativo, falha de restore e rollback; teste de integração real e cutover ficam gates separados.
