# Conversa no terminal (candidato local)

Execute `forge` para abrir diretamente no projeto atual. Veja [entrada nativa](native-command.md)
para instalação, descoberta de projeto e estado automático. O comando `chat` abre uma interface Illumitech Forge dentro do terminal. A UI web continua
em `workbench`. Este incremento ainda precisa de publicação para estar disponível via Cargo.

```sh
# Use um diretório de estado fora do repositório autorizado.
illumitech-forge --state-dir /caminho/externo/forge-state chat --root /caminho/do/projeto
```

É necessário terminal interativo e cliente Codex nativo instalado e autenticado. O Forge
herda a conta e o modelo desse cliente, sem fallback de API. A ponte é a mesma do workbench:
`read-only` e `on-request`; nenhuma permissão é concedida só por entrar no chat.
A primeira mensagem cria uma sessão; sessões salvas são retomadas explicitamente.
Estado e IDs de operações ficam fora de Git, enquanto o histórico completo pertence ao cliente nativo.
Web e terminal usam a mesma proteção do diretório de estado; encerre uma interface antes de
abrir a outra nesse mesmo diretório.

| Comando ou tecla | Ação |
| --- | --- |
| Enter | Enviar mensagem; aguarda o turno atual antes de outro envio |
| Alt+Enter (ou Shift+Enter quando suportado pelo terminal) | Nova linha |
| Setas, Home, End, Backspace e Delete | Editar rascunho Unicode |
| Mouse/trackpad ou PgUp / PgDn | Percorrer a conversa; a leitura não é deslocada por novas saídas |
| Ctrl+Home / Ctrl+End | Ir ao início / retomar acompanhamento da resposta mais recente |
| Esc ou Ctrl+C durante um turno | Solicitar interrupção nativa |
| Ctrl+C sem turno | Limpar rascunho |
| Alt+1…9 | Enviar uma das decisões nativas exibidas, na ordem apresentada |
| `/sessions` | Listar apenas sessões autorizadas deste projeto |
| `/resume` ou `/resume N` | Retomar a sessão atual ou uma sessão numerada |
| `/new` | Criar outra conversa |
| `/reconnect` | Reconectar explicitamente, sem reenviar mensagens |
| `/help` | Mostrar ajuda |
| `/quit` ou Ctrl+Q | Sair e restaurar o terminal |

O painel mostra resposta em streaming, atividade nativa, estado e tokens observados (ausência
não significa zero). Texto de modelo/entrada é tratado como texto, sem executar escapes ANSI.
O rascunho é preservado quando um envio falha ou seu resultado é desconhecido; não há retry
automático. Interromper não garante encerrar terminais nativos em segundo plano.

Esta primeira interface oferece conversa e permissões. Criação de specs, planejamento e
controles de releases continuam nos comandos correspondentes e no workbench web. Não há
alegação de equivalência com todos os recursos do cliente nativo. Uma resposta concluída ou
uma decisão de ferramenta nunca aprova um gate ou release Forge. Windows continua experimental.

## Leitura e descoberta de recursos (FORGE-038)

As boas-vindas usam uma marca ASCII Illumitech; a composição se adapta ao tamanho do terminal.
Respostas exibem títulos, ênfase, listas, tabelas e código como Markdown no terminal. A fonte
original permanece no histórico nativo. HTML e imagens não são executados ou carregados;
caracteres de controle não viram comandos do terminal. Ferramentas ficam resumidas por padrão,
com Ctrl+O para alternar todos os detalhes. Falhas e permissões continuam visíveis.

Para inspecionar individualmente no próprio terminal, pressione **F2** ou digite
`/activities`. Use ↑/↓ para selecionar uma chamada ou arquivo, **Enter**, **Espaço**
ou **Ctrl+O** para mostrar/ocultar seus detalhes, e ←/→ para recolher/expandir.
Abra a atividade de arquivos para selecionar e expandir cada diff separadamente.
PgUp/PgDn percorrem saídas longas; Home/End selecionam a primeira/última atividade
visível. **Esc** ou **F2** retorna à conversa com o rascunho preservado. O painel
funciona durante a resposta e mantém seleção e expansão nas atualizações; trocar
de sessão limpa essa seleção. Essas teclas não executam ferramentas nem aprovam gates.

Chamadas MCP mostram `servidor.ferramenta`; expanda para consultar argumentos,
resultado e erro separadamente. Comandos preservam diretório, código de saída e saída.
Mudanças em arquivos mostram caminhos, diffs e contagens dos hunks recebidos.
No Workbench, cada atividade oferece **Mostrar detalhes / Ocultar detalhes**, e cada
arquivo abre seu próprio diff. Enter e Espaço alternam o resumo em foco. Atualizações
mantêm o cartão aberto. Saídas longas têm área de rolagem e limite explícito; dados
excessivos na ponte são marcados como parciais. Uma atividade concluída ou um diff
não substitui os gates nem o aceite do Forge.

Digite `/` para ver os comandos realmente disponíveis e filtrar pelo nome. ↑/↓ selecionam,
Tab completa e Enter confirma. Esc recolhe o menu, preservando o texto. F1 ou `?` com entrada
vazia abre ajuda; `/status` mostra conta, projeto, modelo e tokens observados, sem inferência.
O Forge não inventa saldo, custo, porcentagem de contexto ou previsão a partir desses valores.
O cronômetro de resposta mede somente o intervalo observado nesta interface.

↑/↓ recuperam pedidos anteriores quando o composer tem uma linha; em texto multilinha movem
o cursor. Ctrl+R ou `/history` pesquisa os pedidos da sessão atual. Enter traz o resultado ao
composer para editar, sem enviar; Esc cancela a busca e preserva o rascunho. Esses controles
não criam arquivos de histórico: a lista em RAM é limitada a 50 pedidos e 256 KiB e continua
restrita à sessão autorizada.

| Atalho adicional | Ação |
| --- | --- |
| Ctrl+A / Ctrl+E | Início / fim da linha |
| Ctrl+U / Ctrl+K | Recortar antes / depois do cursor na linha |
| Ctrl+W | Recortar a palavra anterior |
| Ctrl+Y | Recuperar o trecho recortado ou rascunho limpo por Ctrl+C |
| Ctrl+J | Nova linha, inclusive em terminais sem Shift+Enter |
| Ctrl+L | Redesenhar sem apagar a conversa |
| F1 ou `/help` | Ajuda visual, com PgUp/PgDn |
| `/status` | Informações locais e tokens observados |
| `/exit` | Mesmo efeito de `/quit` |

Referências de interação, consultadas em 31/10/2026:
[comandos do Codex](https://learn.chatgpt.com/docs/developer-commands#built-in-slash-commands)
e [modo interativo do Claude Code](https://code.claude.com/docs/en/interactive-mode).
Essas referências não significam suporte a todos os comandos daqueles clientes: este incremento
não adiciona troca de modelo/permissões, compactação, shell direto ou execução de skills por `/`.

## Preservação da leitura (FORGE-056, 1.4.1)

O histórico permanece dentro da área rolável do Forge. Ao subir, o texto fica ancorado enquanto a resposta cresce; Ctrl+End volta ao fim. Redimensionar preserva a linha lógica lida. Cabeçalho e rodapé compactos deixam mais espaço para a conversa, e o editor cresce com o rascunho. Em terminais que reservam seleção ao aplicativo, use Shift ao selecionar/copiar texto. A captura do mouse é restaurada ao sair. Esta correção não muda a retenção do histórico no cliente nativo nem equivale ao scrollback externo do emulador.
