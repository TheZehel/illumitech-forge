# Plano F064

Base local: 7df888d (candidata 1.5.0), preservada em worktree independente.
Reutilizar Context e review::plan para revisões, limites e schema de achados existentes.
Adicionar `modules list` e `modules qa-plan MANIFEST --level light|standard|intensive`.
Não alterar contratos de execução ou o CLI instalado neste incremento.

Prioridade atual do usuário: feature primeiro, depois QA.
Ordem: planejamento feature → execução incremental de feature com validação →
QA executável → loop corretivo/checkpoint → refatoração → CI/CD. Cada etapa exige evidência própria; a lista não é promessa
 de implementação concluída. Integrar ferramentas por stack em etapas posteriores.

Validação: testes de CLI com Git temporário, manifest externo e estado ausente;
nível inválido rejeitado; níveis monotônicos; revisão presente; evidência não fabricada.
Executar fmt, clippy, testes do workspace e cargo doc. Validação local não é piloto nativo
nem aceite do ledger externo. Não publicar ou instalar automaticamente.

## Execução feature

Adicionar `modules feature prepare|start|status|resume|checkpoint`. Usar o Request,
Candidate, avaliadores e SQLite já existentes; não duplicar o scheduler ou o ledger.
Preparação lê e valida um único request e reutiliza prepare::prepare. A admissão
feature exige proposta e correspondência dos checks com cada critério. Start/resume
revalidam o candidato como feature antes de usar o workflow existente.

Checkpoint é uma projeção somente leitura para handoff de contexto: objetivo, DAG,
fonte preparada/atual, hashes, política de contexto, limites e evidências atuais.
Não é um importador de estado nem um mecanismo de compactação automática.

Regressões com Codex simulado e processos/checks reais: dependências, falha de check,
prosa sem entrega, escopo alterado, consentimento incorreto, gates humanos e retomada
sem replay. Não chamar essas fixtures de piloto com conta/modelo nativo.

## Interface interativa

Seletor inicial e comando `/mode` no terminal Ratatui. Modo é estado de interface,
sem ampliar permissões ou registrar consentimento. Feature transforma texto livre em
planejamento estruturado existente; QA usa planejamento local e exige o mesmo projeto.
O painel mostra disponibilidade e profundidade real, e a autorização é preparada
no editor após revisão, sem copiar manualmente hashes. Duas ações explícitas separam
abrir a autorização e executar. Validar PTY com fixture antes da instalação local.
Preservar os binários instalados para rollback e verificar PATH/readback após atualização.
