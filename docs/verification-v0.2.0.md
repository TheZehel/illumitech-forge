# Verificação da candidata v0.2.0

Fonte dos binários: `3eab629f07b599b012484299794e5621bbf0879c`. Os commits posteriores desta
candidata registram documentação e evidências; os inputs executáveis do build permanecem idênticos.

| Gate | Resultado | Evidência |
|---|---|---|
| fmt, Clippy, testes e documentação | passou | [CI macOS/Linux](https://github.com/TheZehel/illumitech-forge/actions/runs/40765582769), 44 testes por plataforma |
| Specs | passou | 13 specs e 29 IDs permanentes |
| Dependências/licenças | passou | 211 pacotes, avisos completos, OSV sem achados no lockfile consultado |
| Segredos | passou | diff escaneado com saída redigida, nenhum achado |
| Revisão independente R3 | passou no escopo corrigido | staging cancelável, prontidão/limpeza dos testes, contratos de daemon/publicação/importação |
| Piloto GitHub sem memória | passou | [PR #1](https://github.com/TheZehel/illumitech-forge/pull/1), uma execução e reuso, autoria humana, pausa/retomada |
| Execução nativa via Forge 0.2 | passou | [gate externo e worktree preservado](native-parity-acceptance.json) |
| Publicação da própria implementação | passou | [draft PR #2](https://github.com/TheZehel/illumitech-forge/pull/2) aberto e reutilizado pelo Forge final |
| Pacotes/checksums | passou | [recibo de distribuição](distribution-v0.2.0.json), macOS demo completo, Linux --version em Ubuntu 24.04 isolado |
| Release GitHub e configuração cotidiana | aguardando aceite humano da revisão | nenhuma release nova, merge ou troca global realizada |

A primeira revisão encontrou cancelamento incompleto da publicação e destinos Git efetivos não
conferidos. A segunda reproduziu testes frágeis sob concorrência e staging síncrono sem timeout.
As correções incluem SIGTERM durante `gh` e durante um filtro Git bloqueado, `pushurl`, rewrites,
prontidão confirmada e limpeza de subprocessos. A suíte local repetida, R3 e CI final passaram.

A importação preserva contratos explícitos do Loop. Para tarefas Spec Kit, prioridades e dependências
podem ser declaradas em metadados; não se infere uma DAG a partir de prosa ou do marcador `[P]`.
Tarefas antigas concluídas não se tornam evidência atual. IDs sem identificador explícito usam hash
de conteúdo, conforme documentado.

O PR #1 foi criado draft e reutilizado nesse estado. Depois foi marcado pronto para revisão por
uma mudança externa; o Forge bloqueou a repetição e não desfez essa mudança. O PR #2 permanece
separado da fixture e contém a implementação real. Não houve merge automático.

Os arquivos operacionais, logs completos e caminhos locais ficam fora do repositório. Os pacotes
contêm a documentação da revisão de build; este ledger registra o resultado posterior da CI.
O ledger v0.1.0 permanece como snapshot histórico. Claude continua experimental; ai-memory é opcional.
