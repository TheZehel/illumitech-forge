# Paridade operacional e adoção — 0.2.0

A v0.1.0 publicada permanece disponível. Esta entrega fecha as lacunas necessárias para adoção
no fluxo escolhido: fila supervisionada, draft PR opt-in, expansão Spec Kit e publicação genérica.
O Loop Python continua operacional. Nenhum manifest ou estado legado é migrado automaticamente.

| Fluxo | Forge 0.2 | Limite |
|---|---|---|
| Execução de tarefas | DAG, prioridade, limites, locks e evidência | Um escritor por worktree |
| Daemon | Fila serial opt-in, heartbeat, pausa persistente, sinais | Primeiro plano; sem instalador launchd/systemd |
| Draft PR | Commit humano, push e draft opt-in, journal e reavaliação | Um repositório por contrato; sem merge |
| Spec Kit | Importa pendentes com ID permanente ou hash legado | Candidato revisável; gates/permissões não são inferidos |
| Publicação | Tags/assets genéricos e checksums, aceite humano | Repositório GitHub privado, um alvo por contrato |
| ai-memory | MCP opcional; bloqueio somente com required true | Não é necessário instalar memória para usar o Forge |
| Clientes | Codex validado na primeira entrega | Claude experimental; sem troca silenciosa de API |
| Observabilidade | SQLite, eventos, logs limitados e report | Integrações opcionais do Loop, como LangSmith, não são equivalentes |

## Preparação de um projeto

Crie uma branch em worktree isolado e mantenha contratos, estado e avaliadores em diretórios
externos ao repositório gravável. Configure sua identidade Git humana e confira o remoto.
Instale/autentique `gh` somente se usar a publicação GitHub; tarefas locais não dependem dele.

```sh
git worktree add ../project-feature -b feat/project-feature
illumitech-forge release import-spec --root ../project-feature \
  --spec 001-feature --release-id project-feature --output ../operations/candidate.yaml
```

Revise o candidato: executor/permissões, critérios executáveis, dependências e prioridades.
IDs explícitos nos documentos sobrevivem à movimentação de linhas. IDs derivados do texto de
itens antigos mudam se esse texto mudar: atribua IDs permanentes antes de editar o backlog.
Dependências e prioridades podem ser declaradas na mesma linha:

```markdown
- [ ] T002 Implementar <!-- forge: {"dependencies":["T001"],"priority":10} -->
```

Sem metadados, a prioridade é 100 e não há dependências inferidas do texto. As dependências e
prioridades explícitas do manifest Loop são preservadas. A importação não executa tarefas nem
transforma checkboxes antigos em aceites.

Para habilitar o daemon, acrescente ao manifest revisado:

```yaml
automation:
  enabled: true
  require_clean_worktree: true
  # Omitir publish mantém commit/push/PR desativados.
  publish:
    repository: owner/project
    branch: feat/project-feature
    base_branch: main
    remote: origin
    draft: true
```

```sh
illumitech-forge --state-dir ../operations/state release inspect ../operations/queue/feature.yaml
illumitech-forge --state-dir ../operations/state release run ../operations/queue/feature.yaml --dry-run
illumitech-forge --state-dir ../operations/state daemon --releases-dir ../operations/queue --dry-run
illumitech-forge --state-dir ../operations/state daemon --releases-dir ../operations/queue --once
illumitech-forge --state-dir ../operations/state daemon-control status
```

Use `daemon` sem `--once` para supervisão contínua em primeiro plano. `daemon-control pause`
pausa a seleção de novas releases; `daemon-control resume` libera a fila. Uma execução ativa pode
ser controlada por `pause`, `resume` e `cancel` com seu manifest. SIGINT/SIGTERM encerram o daemon
e cancelam os processos ativos de execução ou publicação. Uma requisição já aceita pelo GitHub
não pode ser desfeita por um sinal; a retomada confere o PR antes de repetir a publicação. O daemon não se instala como serviço nem inicia ao login.

## Recuperação e revisão

Falhas deixam um bloqueio persistente na fila, sem repetir automaticamente publicação ou aceites.
Leia `report` e verifique processos/arquivos. Para retomar uma execução com mudanças materiais,
use `resume MANIFEST --reconcile 'motivo revisado'`; isso invalida as evidências anteriores.
Para falha apenas na publicação, use `release draft-pr MANIFEST`: o journal permite recuperar o
commit e reutilizar o draft correspondente. Depois, `daemon-control retry --release ID --reason
'motivo'` remove apenas o bloqueio da fila. Nenhum desses comandos aprova gates humanos.

Se o manifest tiver gate humano, o commit automático exige novo aceite na fingerprint resultante.
Execute `release approve`, `resume`, e então `release draft-pr`. Uma aprovação anterior ao commit
não é transferida. Mudanças no PR remoto exigem reconciliação manual.

Para criar uma release GitHub, revise separadamente `release publish --dry-run`. O comando real
exige tag remota no commit avaliado, aceite humano atual e arquivos conferidos por SHA256SUMS.
Não existe publicação automática de release ao concluir uma tarefa ou draft PR.

## Evidências desta entrega

Os testes de paridade usam Git e subprocessos reais com um endpoint `gh` simulado para injetar
falhas determinísticas. Isso verifica contratos locais, não substitui o piloto no GitHub.
Piloto remoto [#1](https://github.com/TheZehel/illumitech-forge/pull/1) passou com autoria humana,
sem memória, uma execução da tarefa e reutilização do mesmo commit/PR. Pausa e retomada passaram.
[Recibo sanitizado](parity-pilot.json). A revisão inicial detectou cancelamento da publicação e
remotos efetivos não verificados; as correções têm regressões para SIGTERM, pushurl e URL rewrites.
A suíte local passou com 43 testes. CI macOS/Linux e revisão das correções ainda estão pendentes.
A configuração global e o roteamento dos projetos permanecem no Loop até esses aceites.
