# P0 — UI Illumitech para usar Codex pelo Forge

Status: Discovery. Prioridade: segunda frente do roadmap.
Área: interface, sessões e integração do cliente. Repositório: Illumitech Forge.

## Objetivo

Usar o Codex a partir de uma interface própria da Illumitech: selecionar projeto, conversar,
acompanhar execução, revisar mudanças e aprovar ações. Specs, tarefas, gates e consumo devem
ficar próximos da conversa, permitindo executar o fluxo Forge sem operar o terminal.
Experiência com identidade Illumitech, sem presumir reprodução de todos os recursos do app Codex.

## Contexto e caminho técnico candidato

O dashboard 0.3.0 consulta SQLite e não possui chat nem endpoints de controle. O executor
atual usa `codex exec`; uma conversa interativa é uma integração nova, com ciclo de vida próprio.

A documentação oficial apresenta Codex App Server como interface para clientes ricos,
com histórico, autenticação, eventos e aprovações; expõe também atualização de uso de tokens.
O CLI local 0.166.0 oferece `codex app-server`, marcado experimental. Esta é a opção candidata
para um piloto, não prova de compatibilidade integral ou paridade com o app.
[Documentação oficial, consultada em 2026-10-30](https://developers.openai.com/codex/app-server/).

## Experiência proposta

- Navegação por projetos/worktrees, sessões e releases; chat central com streaming.
- Histórico com retomada, interrupção e estado de conexão explícito.
- Comandos, ferramentas, diffs e resultados visíveis conforme suas permissões e capacidades.
- Aprovações contextualizadas: ação, escopo, motivo e opções reais permitidas pelo cliente.
- Painel Forge com plano, tarefa atual, dependências, gates e evidências da revisão avaliada.
- Consumo observado e orçamento do economizador, distinguindo estimativas e dados ausentes.
- Estimativa de duração após separar specs: faixa, premissas, tempo restante e bloqueios;
  compartilhar o contrato do [estimador de duração](duration-estimator.md).
- Layout Illumitech com logo existente, laranja `#ff7a1a`, grafite e verde-petróleo;
  navegação por teclado, contraste e estados de carregamento, falha e reconexão.

O primeiro alvo é uso local no Mac. Web local versus aplicativo desktop, stack e empacotamento
serão decididos após o piloto. Windows acompanha seus gates próprios antes de suporte estável.
Não expor o App Server diretamente a uma página ou porta pública. Definir autenticação da
interface, ponte local e fronteiras de acesso antes de habilitar escrita.

## MVP e aceites

1. Abrir projeto autorizado, detectar versão/capacidades e conectar ao cliente local.
2. Identificar autenticação ativa ou conduzir o fluxo suportado; preservar seleção autorizada
   de conta/modelo/permissões, sem pedir tokens no chat ou migrar para API paga silenciosamente.
3. Criar e retomar conversa, mostrar eventos e interromper trabalho; testar reconexão sem
   duplicar uma ação cujo resultado ainda é desconhecido.
4. Exibir e responder aprovações reais; recusa permanece recusa. Falha de interface não aprova.
   Aprovação de ferramenta no Codex e aceite de evidência no Forge são decisões diferentes.
5. Executar tarefa descartável com gate externo e revisão somente leitura; manter chat,
   estado de execução e conclusão verificável claramente distintos.
6. Exibir consumo disponível; cancelamento e falha não aparecem como conclusão bem-sucedida.
7. Testar isolamento de projetos, acesso a arquivos, logs redigidos e recuperação após fechar a UI.

## Dependências e sequência

Economizador fornece o contrato de métricas, sem bloquear o primeiro protótipo visual.
Começar por spike de protocolo e compatibilidade, depois protótipo de navegação com estados
reais, integração de uma sessão e por fim comandos Forge. Não ampliar o servidor de inspeção
atual com escrita sem um contrato próprio de autenticação, permissões e recuperação.

Skills continuam procedimentos especializados acessíveis conforme o cliente permitir.
Plugins, conectores, automações, sessões de outros clientes e memória precisam de avaliação
individual; não anunciar que todos os recursos atuais do Codex estarão disponíveis no MVP.
Nenhuma instalação de hooks ou exigência de ai-memory faz parte da UI básica.
