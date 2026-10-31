# FORGE-020 — Estimativa de duração do loop

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Ainda não publicada ou instalada como substituta da 0.3.0.
Escopo original desta spec: estimador primeiro. Economizador e UI são implementados pelas specs 022/023 do programa.
Base: docs/features/duration-estimator.md. Memória opcional; sem publicação ou Actions.

## Contrato e aceites

- F020-AC1: após validar o plano, inspect e dry-run incluem `duration_estimate` no envelope
  JSON schema_version 1. Campos mínimos: schema_version, status, basis, confidence,
  remaining_seconds (objeto lower/upper ou null com motivo), assumptions, counts
  (specs/tasks/gates). Identificar a fase e a revisão/hash de plano a que a previsão pertence.
  A saída humana comunica a faixa e a incerteza em texto compreensível.
- F020-AC2: estimativa determinística sem chamadas de LLM. Ausência de histórico usa faixas
  heurísticas documentadas e confiança baixa, nunca timeout como duração provável. Contar a
  execução serial real, dependências, tentativas como risco e gates inclusive reavaliação final.
  Não anunciar paralelismo que o engine não executa; contabilizar gates repetidos por tarefa.
- F020-AC3: instrumentar durações de execução e gates, incluindo retries/falhas/cancelamentos,
  sem apagar registros anteriores na tentativa seguinte. Estatísticas com amostra e contexto
  compatível; sem reutilizar IDs de outra raiz de projeto ou modelo/configuração conhecidos.
  Configuração efetiva não observável deve ser declarada: não afirmar compatibilidade forte.
- F020-AC4: status/report mostram previsão corrente, tempo ativo observado e a faixa restante.
  Pausa, espera humana e bloqueio não produzem ETA de relógio fictício; marcar espera e
  tempo restante condicional. Atualizar após tarefa/gate e na consulta de execução em andamento;
  não subtrair tempo de aprovação do trabalho ativo. Nunca tornar tempo negativo.
- F020-AC5: persistir previsão inicial e revisões com manifest hash e contexto efetivo;
  report compara com o realizado. Eventos e transições ficam transacionais. Mudança material
  exige a reconciliação existente; preservar histórico da execução anterior de forma auditável.
  Crash/orfandade e dados legados não podem fabricar duração conhecida ou sucesso.
- F020-AC6: inspect/dry-run/status/report somente leitura: estado ausente não é criado.
  Histórico e queries limitados; dados ausentes/corrompidos ou incompatíveis são identificados
  sem inventar amostras. Não alterar validação, controles, locks, aprovações ou saídas de clientes.
- F020-AC7: testes cobrem defaults, contagem serial/gates, histórico e isolamento, estado legado,
  rejeição de manifest alterado, pause/resume, human wait, cancel/timeout, read-only inspection,
  persistência de previsão inicial, falhas/retries e erro/cobertura sem metas inventadas.
  Todos os checks AGENTS e um teste externo de CLI real devem passar.

## Limites

Faixas são previsões, não garantias. Sem suporte a calendário, disponibilidade externa ou
promessa de acurácia. Sem telemetria remota, credenciais, nova dependência, mudança de modelo,
ativação de serviço, push, merge, publicação ou bump de versão nesta tarefa.
Não usar tokens/segundos como conversão universal. Não implementar economizador nem App Server.
