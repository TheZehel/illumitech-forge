# Estimativas de duração (FORGE-020)

O Forge apresenta uma faixa de **trabalho ativo restante**, calculada localmente, depois de
validar o manifest. Não chama agentes, não consulta um modelo e não converte tokens em tempo.
Mantém os executores, permissões e modelos nativos já configurados. A faixa não é um prazo,
probabilidade calibrada, limite de execução ou evidência de aceitação.

## Consultas

```sh
illumitech-forge --json release inspect release.yaml
illumitech-forge --json release run release.yaml --dry-run
illumitech-forge --json status release.yaml
illumitech-forge --json report release.yaml
```

Os quatro comandos são somente leitura. Sem estado, não criam banco, diretório ou tabelas.
Inspect/dry-run podem abrir um banco existente em modo somente leitura para consultar histórico;
continuam utilizáveis com histórico ausente, legado ou corrompido, indicando a degradação.
Status/report sem execução retornam `not_run` e a previsão do plano. Um banco ilegível ou snapshot
inválido produz erro explícito em status/report, sem tentar reparar o estado.

A saída humana inclui a faixa em segundos e a incerteza. JSON preserva o envelope
`schema_version: 1` e acrescenta `duration_estimate`, também com versão 1. O mesmo cálculo
alimenta inspect, dry-run, execução, status e report. A execução apresenta a prévia em stderr
antes de iniciar os subprocessos; stdout JSON continua sendo um único envelope.

## Faixas iniciais

Sem pelo menos três medições bem-sucedidas por contexto/fase, usam-se estas heurísticas:

| Unidade | Inferior (s) | Superior (s) |
| --- | ---: | ---: |
| Execução de tarefa `command` | 5 | 120 |
| Execução de tarefa Codex/Claude | 60 | 900 |
| Invocação de gate | 1 | 120 |
| Verificação de artefato | 0 | 2 |
| Preparação local do plano | 2 | 30 |
| Overhead local por tarefa pendente | 0 | 10 |

São premissas amplas, não benchmarks. O padrão permanece serial: somam-se as unidades na execução. Com
`execution.max_parallel > 1`, o estimador simula as ondas reais do DAG, os worktrees
e o limite compartilhado de concorrência; gates e risco de retries seguem conservadores.
Veja [o contrato M07](m07-operating-contracts.md). Conta-se cada referência de gate por tarefa, mais
uma avaliação final de todas as tarefas. Gates compartilhados não são deduplicados.
Verificações humanas têm duração de espera desconhecida e não recebem zero segundos de espera.

O inferior assume uma tentativa bem-sucedida. O superior acrescenta o orçamento de retries
permitido, repetindo execução e verificações locais por tentativa adicional. Timeouts não são
usados como duração provável. Tentativas consumidas reduzem o risco futuro; avaliações repetidas
após retomada e aprovação voltam a compor o trabalho pendente. Um overrun reduz apenas a previsão
da unidade em andamento: não consome a faixa de tarefas/gates posteriores. A unidade excedida
mantém margem superior de metade de sua faixa superior, no mínimo um segundo. Não cancela trabalho.

`counts` informa specs referenciadas por repo, tarefas, definições de gates, invocações de gate
(incluindo a avaliação final, sem retries), verificações de artefato, aprovações humanas e orçamento
de retries. `manifest_hash`, `execution_context_hash`, fase e instante Unix em milissegundos
identificam a previsão. O hash de contexto inclui raízes canônicas, contrato e plataforma;
nenhum caminho ou conteúdo do contrato é copiado para o registro de medição, apenas seu digest.

## Histórico e confiança

A consulta lê no máximo as 200 medições mais recentes, com limite de 64 KiB por registro.
O contexto de cada unidade inclui raiz canônica do projeto, contrato completo da tarefa,
executor, modelo/permissão declarados, specs, fase, comando e inputs fixados do gate, plataforma,
arquitetura e versão do Forge. IDs iguais em outra raiz não compartilham amostras. Uma mudança
conhecida de contrato/configuração impede o reaproveitamento. Releases distintos podem usar
histórico compatível na mesma raiz e banco. Um banco separado começa sem histórico.

Com pelo menos três amostras bem-sucedidas por unidade, a faixa empírica é metade do mínimo
até duas vezes o máximo, com largura mínima de um segundo. As outras unidades conservam as
heurísticas. `basis` distingue `heuristic` e `local_history_and_heuristics`; `status` distingue
`preliminary`, `estimated` e `unavailable`. A confiança é baixa, ou média quando todas as
unidades têm amostras suficientes e usam comandos. Não há confiança alta.

A configuração efetiva de um cliente nativo não é inteiramente observável: perfil, modelo
herdado, versão do cliente, ambiente e cache podem mudar sem alteração do manifest. Até com
modelo declarado, a compatibilidade é parcial e a confiança permanece baixa.
`configuration_observability` explicita esse limite. O Forge não lê credenciais nem executa o
cliente para descobrir configuração. Para comandos, ambiente, ferramentas e cache também são
fontes de variação não observadas.

Falhas, bloqueios, timeout, cancelamento e intervalos incompletos permanecem registrados e
contados separadamente; não são amostras de sucesso. Histórico inválido é descartado da
calibração com motivo explícito, sem inventar amostras. O limite pode omitir registros antigos;
a ausência de correspondência na janela não prova que nunca houve execução comparável.

## Medições, retomada e espera

Cada execução e cada verificação têm sequência, tentativa, fase, resultado, início Unix e duração
monotônica medida. Gates finais são identificados separadamente. Overhead ativo da invocação
(preparação e bookkeeping) é registrado ao parar, descontando intervalos já medidos para evitar
dupla contagem. O total persiste entre retomadas. Durante execução, o tempo exibido soma os
intervalos já medidos e o intervalo corrente provisório; o overhead corrente é acertado na parada.

Status/report projetam um intervalo aberto usando o relógio de parede e o PID do supervisor,
sem escrever. Essa parcela é provisória: relógio invertido, intervalo além do timeout mais dez
segundos ou supervisor morto tornam a previsão indisponível. PID vivo não comprova progresso
nem sucesso; reutilização de PID é uma limitação da projeção local. Heartbeat não é evidência.
Após crash, intervalos abertos não recebem duração completa nem resultado bem-sucedido.

Pausa, espera humana, bloqueio e cancelamento indicam `waiting_for`, `conditional: true` e
`wall_clock_eta: null`. O intervalo de espera fora da execução não é subtraído do trabalho ativo.
O restante é condicional à intervenção/retomada; cancelamento ainda exige reconciliação pelo
contrato existente. Cancelamento ou orçamento esgotado tornam a faixa indisponível até
reconciliação, sem apagar o tempo medido. Números restantes nunca ficam negativos. Manifest, worktree ou artefato
alterados invalidam a projeção do estado parado e exigem a reconciliação existente. Durante um
subprocesso vivo, alterações esperadas do worktree não escondem sua projeção; não validam evidência.

## Persistência e relatório

SQLite continua na versão 1 com tabelas aditivas `duration_measurements` e `duration_forecasts`.
A criação ocorre somente em operações de escrita autorizadas. Snapshot do run, evento, medição
e revisão de previsão são gravados na mesma transação. A previsão inicial é imutável no run;
cada transição guarda uma revisão com motivo. Medições finalizadas não são sobrescritas por
uma tentativa seguinte. A reconciliação arquiva o run anterior no evento, incrementa a geração
e conserva as medições e previsões anteriores. Não modifica o ledger externo de tarefas.

Report retorna as últimas 100 revisões e até 1.000 eventos, em ordem cronológica entre os eventos
retidos. A previsão inicial permanece acessível no run mesmo quando sai dessa janela.
`duration_comparison` compara o ponto médio inicial com o ativo observado (`error_seconds`) e
indica se o total caiu na faixa (`coverage`) apenas para execução concluída com medições completas.
Falha/cancelamento têm comparação parcial e não recebem cobertura de conclusão. O relatório
separa resultado operacional e métricas de duração. Estados legados não fabricam previsão inicial,
duração ou sucesso: mostram indisponibilidade; uma retomada pode medir novos intervalos sem
atribuir duração conhecida ao passado.

Não existe meta numérica de acurácia. A previsão inicial usa apenas dados disponíveis antes da
execução; é a referência preservada para comparar amostras posteriores. Revisões podem incorporar
medições da execução corrente e não devem ser tratadas como avaliação fora da amostra. Esses
números não substituem gates externos vinculados à revisão, aprovação humana ou aceitação.
