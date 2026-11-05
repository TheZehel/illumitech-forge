# F064 — Módulos de desenvolvimento

## Intenção
Separar harness (Codex inicialmente) de módulo (QA, feature, refatoração e CI/CD).
Priorizar feature, depois QA, conforme a última orientação do usuário.
O primeiro incremento expõe planejamento feature sobre o contrato existente e preserva
o planejamento QA em três profundidades para a etapa seguinte.

## Primeiro incremento
- F064-AC1: catálogo diferencia disponibilidade real de roadmap.
- F064-AC2: plano QA associado às revisões e ao manifest existentes, sem chamadas nativas,
  scanners, scripts do repositório, gravação de estado ou autorização de publicação.
- F064-AC3: níveis light, standard e intensive ampliam cobertura sem ampliar permissões.
- F064-AC4: verificações pendentes não recebem evidência, aprovação ou aplicabilidade presumida.
- F064-AC6: feature valida requisitos, DAG e aceites usando o planejador existente;
  comandos propostos não são executados e critérios ausentes bloqueiam o plano.
- F064-AC5: loop e checkpoint aparecem explicitamente como design, sem simular execução.

## Incrementos seguintes
QA executável: descoberta de stack; seleção explícita de ferramentas; avaliadores externos
fixados; achados com revisão, severidade, confiança e evidências; lacunas como not_run,
not_applicable justificado, unavailable ou failed, nunca passed por falta de ferramenta.
Pesquisa de vulnerabilidades deve guardar origem e data da consulta. Rede não é implícita.
Correção e revalidação compartilham orçamento e limite de tentativas; parar em falha repetida.

Feature: objetivo, critérios de aceite, tarefas pequenas, implementação e QA.
Refatoração: baseline de comportamento, testes de caracterização, mudanças pequenas e QA.
CI/CD: planejamento, build, QA, artefatos imutáveis, aprovação vinculada ao destino/revisão,
deploy, verificação e decisão de rollback. Configuração de pipeline não autoriza publicação.

Continuidade: checkpoint externo ao contexto antes de compactar, com objetivo, decisões,
revisões/hashes, evidências, tarefas pendentes, trabalho incompleto e orçamento restante.
Retomada revalida fontes e não transforma resumo em aceite. Tokens desconhecidos permanecem
 desconhecidos; não prometer redução de custo antes de medir entregas aceitas.

## Segundo incremento — execução feature
- F064-AC7: `modules feature prepare` exige proposta estruturada e mapeamento exato
  de ID/argv/escopo de cada critério para um check externo; nunca adivinhar verificações.
- F064-AC8: start exige digest do escopo preparado. Executar Codex e checks pelo
  workflow existente, respeitando DAG, limites, permissões e gates humanos.
- F064-AC9: falha do check impede concluir a tarefa e iniciar dependentes; prosa
  não substitui evidência. Retomada explícita não duplica trabalho concluído.
- F064-AC10: status/checkpoint projetam o estado externo e evidências sem execução;
  checkpoint inclui objetivo, proposta/DAG, limites, contexto e hashes. Ele não
  concede aceite e não substitui revalidação na retomada.

Compactação automática, medição precisa do contexto disponível e correção automática
continuam fora deste incremento. Uma tarefa corresponde a uma unidade de contexto
controlada pelo workflow; falta de uso observado não equivale a orçamento ilimitado.

## Terceiro incremento — escolha interativa e operação local
- F064-AC11: `forge` abre seletor de modos; `/mode` reabre. Feature, QA planejamento
  e Conversa são selecionáveis; Refatoração e CI/CD ficam explicitamente indisponíveis.
- F064-AC12: texto livre em Feature planeja pelo cliente nativo e aplica a mesma
  admissão de critérios/checks. Planejar não executa; confirmação permanece vinculada
  ao digest. Enter no plano apenas prepara autorização no editor; outro Enter executa.
- F064-AC13: QA encaminha o manifest/profundidade ao planejador local, restrito ao
  projeto selecionado. Não chama LLM/scanner nem cria estado de execução.
- F064-AC14: trocar modo invalida o escopo selecionado na UI e não muda permissões;
  operação em andamento impede a troca. Conversa preserva o comportamento somente leitura.
- F064-AC15: validar renderização, PTY real com cliente simulado e instalação local
  reversível. Distinguir piloto de conta real de fixtures; não publicar implicitamente.

O usuário autorizou tornar a funcionalidade operacional/interativa e atualizar a
instalação local nesta etapa. Isso não autoriza publicação de release ou deploy remoto.
