# Illumitech Forge v0.2.0 — candidato de paridade operacional

Esta versão acrescenta os fluxos necessários para substituir gradualmente o Loop nos projetos
selecionados: daemon opt-in com controles persistentes, draft PR com autoria humana, expansão de
tarefas Spec Kit e publicação de releases sem exigir ai-memory.

- Daemon serial com locks, heartbeat, pausa/retomada, cancelamento e bloqueio de falhas para revisão.
- Commit, push e draft PR somente quando declarados; reavaliação dos gates e aceites após commit,
  journal de recuperação e reutilização do PR correspondente. Nenhum merge automático.
- Conferência dos destinos efetivos do Git, incluindo pushurl e URL rewrites.
- Importação de tarefas pendentes com IDs estáveis e metadados de dependência/prioridade.
- Publicação com tags e assets genéricos, checksums e aceite humano na revisão atual.
- ai-memory opcional em execução e publicação; required true continua bloqueando sem verificação.

44 testes locais, formatação, Clippy, documentação e avisos de 211 dependências passaram.
O piloto remoto sem memória criou e reutilizou o [draft #1](https://github.com/TheZehel/illumitech-forge/pull/1),
com autoria de Allison Yuri e uma execução da tarefa. A execução nativa somente leitura pelo
Forge 0.2 também passou, com preservação do worktree. Os testes incluem interrupção de staging,
URL rewrites, recuperação de journal e invalidação de aceites após commit.
[CI macOS/Linux](https://github.com/TheZehel/illumitech-forge/actions/runs/40765582769) passou com
44 testes por plataforma. Os dois pacotes e checksums foram verificados; o exemplo macOS e a
execução Linux isolada passaram. Fonte dos binários: `3eab629`. Consulte o
[ledger final da candidata](verification-v0.2.0.md). Falta somente o aceite humano da revisão para
publicação privada; nenhuma troca global foi realizada.

A v0.1.0 permanece preservada. Claude continua experimental. Não há instalação de serviço,
migração automática dos projetos ou equivalência com integrações opcionais do Loop como LangSmith.
Consulte [operação e recuperação](operational-parity.md) antes de habilitar automação.
