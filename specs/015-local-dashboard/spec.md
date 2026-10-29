# FORGE-015 — Dashboard web local

## Limites e evidência
Especificação autorizada em 2026-10-29; implementação local disponível, aceitação externa desta frente pendente.
Claude permanece experimental, explicitamente adiado: sem login ou validação nesta entrega.
ai-memory é opcional; somente `required: true` exige disponibilidade e escopo comprovados.
Estado, avaliadores e recibos operacionais ficam fora de repositórios graváveis pelo agente.
Prosa do agente, checkbox e telemetria não substituem gates externos vinculados à revisão.
Esta especificação não autoriza publicação, instalação global, ativação de serviço ou migração de produção.

## Dependências
FORGE-013 (estado e status); FORGE-019 acrescenta observabilidade opcional, sem bloquear a leitura local.

## Intenção e contrato
Oferecer inspeção local em português de releases, tarefas, gates, eventos e status já registrados.
`illumitech-forge --state-dir <externo> dashboard --port <porta>` inicia servidor em primeiro plano,
em `127.0.0.1` apenas (porta padrão 0, porta atribuída informada no terminal). Não iniciar daemon,
abrir navegador nem executar agentes. Usar ativos locais com laranja, grafite e teal; sem CDN/scripts externos.
Rotas GET `/`, `/releases/<id>` e `/health` são somente leitura. IDs são chaves, nunca paths.
Outros métodos retornam 405 e rotas desconhecidas 404; nenhum endpoint de comandos ou arquivos arbitrários.

## Aceites
- F015-AC1: SQLite externo aberto read-only, sem criar/migrar banco. Ausente: estado vazio explícito;
  bloqueado/corrompido: erro sanitizado, sem fallback gravável nem vazamento de paths.
- F015-AC2: lista paginada até 100 itens, eventos até 200 por página e resposta até 1 MiB;
  consultas parametrizadas, cursor estável, snapshot consistente durante escrita concorrente.
- F015-AC3: HTML escapado para todo campo; somente metadados permitidos, sem prompts, logs brutos,
  ambiente ou segredos. CSP restritiva, sem CORS, Host limitado ao loopback/porta efetivos;
  rejeitar Host remoto e Origin estrangeira, inclusive contra DNS rebinding.
- F015-AC4: UI distingue passed/failed/blocked/not_run e heartbeat antigo; teclado, foco visível,
  contraste e layout a 360px/1280px. Nunca representar resultado ausente como sucesso.
- F015-AC5: testes HTTP reais locais cobrem métodos, rotas, XSS, IDs maliciosos, limites, banco
  ausente/corrompido e leitura concorrente; hash do banco preservado em leitura quiescente.
