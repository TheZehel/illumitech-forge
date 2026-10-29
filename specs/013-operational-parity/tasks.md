# Tarefas FORGE-013

- [x] F013-T01: Publicação genérica, memória opcional e caminhos relativos; evidência: regressões de checksums/tag/aceite.
- [x] F013-T02: Prioridade e expansão Spec Kit; evidência: IDs estáveis, dependências e importação sem aceite herdado.
- [x] F013-T03: Daemon persistente opt-in; evidência: lock, pausa, reinício, alterações materiais e dry-run sem escrita.
- [x] F013-T04: Draft PR idempotente com identidade humana; evidência: Git real, falha parcial e PR existente.
- [x] F013-T05: CI, piloto e documentação de adoção; evidência: uso real e comparação com o Loop preservado.

Evidência local: 44 testes, fmt, Clippy sem warnings, cargo doc e 211 dependências com avisos completos.
Piloto remoto: draft PR #1, commit 9352a73, autoria Allison Yuri, sem memória, uma execução, reuso e pausa/retomada.
Revisão inicial detectou cancelamento da publicação e destinos Git efetivos; corrigidos com regressões.
A segunda revisão detectou espera frágil em testes de sinais e staging sem cancelamento; ambos corrigidos.
CI 40765582769 passou com 44 testes por plataforma. Pacotes e piloto final verificados; ver docs/verification-v0.2.0.md.
A execução técnica de T05 está concluída. A publicação v0.2.0 foi confirmada em [recibo posterior](../../docs/expansion-evidence.md). Configuração cotidiana não é inferida desse recibo; novas revisões exigem seus próprios aceites.
