# F065 — QA executável com cobertura verificável

## Objetivo
Continuar F064 com QA operacional no CLI e no seletor de terminal. Primeiro incremento
executa verificações explicitamente revisadas, não promete segurança universal, não
faz correções, instala ferramentas ou dispara scans remotos automaticamente.

## Interface pública requerida
Preservar `modules qa-plan MANIFEST --level ...` e todos os comandos feature existentes.
Adicionar `modules qa discover --root ROOT` e `modules qa prepare REQUEST --root ROOT
--ops-root OPS [--apply]`, `modules qa start ID --root ROOT --ops-root OPS
--approve-scope DIGEST`, `modules qa status ID --root ROOT --ops-root OPS`.

REQUEST JSON estrito schema_version=1, id, title, level (light|standard|intensive), checks.
Cada check: id, coverage_id, argv (array), timeout_seconds (1..600), inputs (mapa de
caminho externo para SHA-256), format opcional (text por padrão, ou sarif).
Pelo menos um check e um input externo fixado por check. Aceitar no máximo 32 checks.
Substituição argv somente `{repo}` como argumento inteiro; nunca interpolação shell.
Comandos shell explícitos continuam sendo código revisado pelo operador, não prosa.

Coverage IDs correspondem aos já exibidos por F064:
light: QA-QUALITY, QA-SECRETS, QA-DEPENDENCIES;
standard também QA-SAST, QA-AUTH, QA-BUILD;
intensive também QA-ADVERSARIAL, QA-THREAT.
Recusar coverage_id desconhecido ou fora da profundidade escolhida.

## Aceites
- F065-AC1: discover apenas lê indicadores de stack; sugere comandos sem executá-los,
  instalar dependências ou atribuir cobertura. Fonte de sugestões permanece explícita.
- F065-AC2: prepare é preview por padrão; --apply materializa candidato imutável fora
  do Git gravável. Resposta inclui status preview/prepared e scope_digest. Não executa checks.
- F065-AC3: start valida digest, fonte atual, identidade do projeto e hashes externos
  antes de qualquer spawn. Rejeitar inputs/estado dentro do projeto, symlinks e traversal.
  Serializar execução por worktree. Nenhuma publicação, tarefa nativa ou reparo implícito.
- F065-AC4: executar subprocessos com timeout, saída limitada/redigida e cancelamento.
  Comandos são autorizados explicitamente e podem gerar artefatos de build. Isso NÃO é
  um sandbox de leitura: detectar alteração de fonte não ignorada e bloquear o resultado,
  sem restaurar ou sobrescrever mudanças. Não alegar imutabilidade imposta pelo SO.
- F065-AC5: persistir journal/resultado fora do root. Publicar `checks` e `coverage` com
  IDs, status e evidências. Cobertura não executada é not_run; fora do nível é not_requested.
  Missing executable é unavailable, timeout/falha/truncamento não são passed. Todos os
  checks de uma cobertura devem passar para ela passar. Lacunas do nível tornam o
  resultado geral degraded (exit 2); falhas/achados tornam failed/blocked (exit 2).
  Apenas todas as coberturas do nível verificadas sem falhas permitem status passed.
  Nunca emitir uma declaração de segurança universal. Ausência de ferramenta não é êxito.
- F065-AC6: format=sarif usa o parser existente, preservando achados e erros do scanner.
  SARIF inválido, erros de ferramenta ou achados não podem gerar QA passed, mesmo com exit 0.
- F065-AC7: fonte/contrato/evaluator alterados tornam status stale/blocked; nunca reutilizar
  aprovação histórica. Duplo start não repete verificações. Interrupção/resultado incerto
  fica explícito e exige novo escopo; sem replay automático ou edição manual de SQLite.
- F065-AC8: QA no seletor passa a operacional com preparação e confirmação separadas.
  Entrada de REQUEST prepara; mostrar argv, cobertura, escopo e lacunas antes de permitir
  start. Preservar entrada do manifest antigo como planejamento-only quando aplicável.
  `/qa-run DIGEST` e `/qa-status [ID]` (ou equivalentes documentados) operam apenas o
  projeto selecionado. F3 e feature continuam funcionando. Não bloquear renderização
  durante checks e não usar a aprovação de feature para QA.
- F065-AC9: exemplos e documentação explicam níveis, limites e ferramentas ausentes.
  Testes cobrem preview, happy path, digest errado, replay, fonte/hash alterados, inputs
  internos, timeout, executável ausente, SARIF e fonte modificada. Gates antigos passam.

## Fora de escopo
Compactação automática, correções automáticas, fuzzing sem contrato, instalação de
scanners, rede automática, novos provedores, CI/CD e refatoração. O comando pode executar
um scanner já instalado e revisado; a disponibilidade real deve ser observada.
