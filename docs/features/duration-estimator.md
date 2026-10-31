# Estimador de duração do loop

Status: MVP local implementado em FORGE-020; publicação pendente.
Prioridade: terceira frente do roadmap, integrada ao planejamento inicial.
Área: planejamento, execução, métricas e UI. Repositório: Illumitech Forge.

## Objetivo

Para cada pedido que gere ou altere um plano de execução, apresentar uma estimativa de duração
assim que o Forge separar specs, tarefas, dependências e gates, antes de iniciar o loop.
Mostrar tempo restante durante a execução e revisar a previsão quando o escopo mudar.
Conversas informativas sem loop não precisam gerar uma previsão de execução.

## Contexto atual

Os manifests descrevem tarefas, dependências, limites e critérios. Eventos e resultados de
subprocessos permitem explorar medições históricas, mas ainda é necessário inventariar quais
intervalos e fases são registrados de forma completa. Timeouts são limites operacionais;
não devem ser apresentados como duração provável. A 0.3.0 não oferece esta estimativa.

## Comportamento proposto

1. Após dividir o pedido, produzir um resumo com quantidade de specs/tarefas/gates, duração
   provável em faixa, confiança, premissas e principais fatores de incerteza. Exibir sempre
   o estado da previsão: estimada, preliminar ou não estimável, com motivo.
2. Considerar preparação de ambiente, implementação, revisão, builds, testes, integração,
   overhead do Forge e margem de retrabalho. Usar execução serial enquanto ela for o modo
   real; quando houver paralelismo, considerar dependências, concorrência e recursos limitantes.
3. Preferir medições locais de trabalhos comparáveis, discriminando tipo de tarefa, cliente,
   modelo/configuração, stack, ambiente, cache e tamanho do escopo quando mensuráveis.
   Na ausência de histórico suficiente, usar premissas explícitas e faixa preliminar ampla.
   Não inventar amostras, confiança estatística ou precisão em minutos.
4. Separar tempo ativo de execução, fila/espera externa e espera humana. Aprovações, login,
   bloqueios e serviços indisponíveis podem tornar o horário final desconhecido. Nesses casos,
   manter a faixa de trabalho ativo restante e indicar a dependência; não contar espera como
   zero nem prometer previsão de relógio. Retomar sem apagar o tempo já observado.
5. Durante o loop, atualizar tempo decorrido, restante e previsão de conclusão quando possível,
   sobretudo após cada tarefa/gate. Reestimar mudanças de escopo, retry, falha, reconciliação
   e invalidação de cache. Preservar a previsão inicial e o motivo de cada revisão.
6. No relatório final, comparar previsão inicial e revisões com o realizado; separar sucesso,
   falha e cancelamento. Calibrar o estimador com esses resultados sem esconder desvios.

O usuário não precisa solicitar a estimativa nem aprová-la em uma etapa adicional. Ela aparece
no resumo do plano e segue o fluxo de autorização já existente. Um prazo solicitado pelo usuário
é uma restrição a comparar com a previsão, não um valor para o qual ajustar artificialmente o ETA.
Retentativas continuam submetidas aos limites e às recusas de permissão existentes.

## Apresentação ilustrativa

Exemplo fictício, não benchmark nem promessa de precisão:

```text
Plano: 3 specs · 7 tarefas · 4 gates
Execução ativa estimada: 25–45 min
Confiança: baixa — projeto sem histórico comparável
Premissas: execução serial, ambiente pronto, uma rodada de revisão
Espera por aprovação: duração desconhecida, fora da faixa ativa
```

CLI, JSON versionado e futura UI devem compartilhar a mesma previsão. Mostrar instante da
estimativa e fase atual. Especificar os campos de duração e seus estados antes da implementação.

## Aceites propostos

- Todo plano executável apresenta previsão ou indisponibilidade justificada após a separação
  das specs e antes da execução; o resumo é renovado após mudança material do plano.
- Faixa, unidade, base histórica/premissas e confiança são explícitas. Limite de timeout e
  previsão provável aparecem como conceitos separados; intervalos desconhecidos não viram zero.
- Casos seriais, futuras dependências paralelas, instalação/build com cache frio, falha/retry,
  pausa, cancelamento, retomada e espera humana têm comportamento verificável.
- Estimativa não cria chamadas adicionais de agente por padrão. Se uma modalidade assistida
  for proposta, contabilizar seu próprio tempo e tokens e compará-la ao método determinístico.
- Histórico, previsão inicial e revisões ficam vinculados ao hash do manifest e configuração
  efetiva, em estado operacional externo ao repositório gravável. ai-memory continua opcional.
- Avaliar erro de duração, cobertura da faixa e viés por tipo de tarefa usando amostras
  posteriores às usadas na calibração. Definir metas numéricas somente após obter baseline.
- Exceder a previsão atualiza a informação ao usuário; não cancela trabalho por si só nem
  transforma evidências pendentes em sucesso. Limites explícitos de execução continuam válidos.

## Dependências e descoberta

Planejamento guiado fornece o plano estruturado; a instrumentação compartilhada com o
economizador fornece medições. A UI exibe as mesmas informações, mas não é pré-requisito
para o primeiro estimador na CLI. Não exige habilitar paralelismo ou memória para funcionar.
Módulos prováveis: contratos de domínio/SQLite, engine, adapters/process, inspect/report e UI.
Primeiro piloto: estimativa determinística baseada em fases e histórico local, acompanhada
de comparação com duração real, sem ampliar os gates nem chamar agentes desnecessariamente.
