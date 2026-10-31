# Programa de execução do Forge até 1.0

Status: implementação local candidata 1.0.0; aceitação final da revisão em andamento.
A execução efetiva agrupou os passos por marco, com gates externos materializados.
O desenho original abaixo é histórico; os blueprints não são evidências de execução.
Consulte [a candidata e seus limites](v1-candidate.md).
Base publicada: 0.3.0. Candidato local herdado: FORGE-020, validado anteriormente com 99 testes.
O candidato foi copiado para worktree isolado; os ledgers anteriores não são transferidos.
Escopo: todas as treze frentes discutidas, com confiabilidade e estabilização como suporte.

## Entregas e ordem

| Marco | Specs | Entrega | Gate de saída |
|---|---|---|---|
| 0.4 | 020, 021, 022 | Consolidar estimador, corrigir protocolo e entregar economizador | Protocolo redigido válido; métricas/contexto; baseline de tokens com qualidade |
| 0.5 | 023, 024, 025 | UI Illumitech, criação guiada e planejamento assistido | Browser e Codex reais; pedido → specs → previsão → execução |
| 0.6 | 026–029 | Templates, controle na UI, orçamento e reuso de evidências | Sem gasto/CI implícitos; aprovações e invalidações preservadas |
| 0.7 | 030, 031 | Paralelismo isolado e revisão independente | Integração final, cancelamento, conflitos e orçamento compartilhado |
| 0.8 | 032 | Issue → specs → execução → draft PR | Piloto remoto declarado, sem duplicação e com autoria humana |
| 0.9 | 033 | Instalação, atualização e recuperação | Backup/restore e atualização interrompida; plataformas avaliadas |
| 1.0 | 034 | Estabilização, contratos e aceitação integrada | Pilotos reais, documentação, inventário, pacotes e revisão final |

Os marcos são candidatos internos: não exigem publicar uma versão intermediária ou pedir
aprovação humana a cada tarefa. Não representam datas prometidas. O DAG detalhado está em
[specs/programs/v1.0.json](../specs/programs/v1.0.json). Cada spec nova tem spec.md, plan.md e tasks.md.
Há **42 passos novos**, mais uma tarefa de revalidação do estimador existente e sete checkpoints.

## Execução sem confirmações rotineiras

O coordenador conduz os marcos em sequência, implementa, revisa, corrige falhas e valida os
resultados dentro deste contrato. Resolve decisões técnicas rotineiras usando o repositório,
medições e ADRs curtas. Não pede ao usuário para escolher bibliotecas, organizar arquivos,
rodar cada teste ou autorizar novamente ações já cobertas pelo escopo.

- Um escritor por worktree até a spec de paralelismo estar validada.
- Até duas tentativas por tarefa, sem ciclos infinitos. Retry não contorna contrato, permissão,
  autenticação ou aceite ausente. Diagnóstico e correções locais pertencem ao coordenador.
- Modelo, conta e políticas do cliente preservados. Nada de fallback silencioso para API paga.
- Checks direcionados por passo; suíte integrada nos checkpoints. Reusar somente prova verificável.
- Se um item já estiver implementado e houver evidência aplicável ao candidato atual, trocar
  reimplementação por validação determinística; ajustar o manifest e revalidar os contratos.
  Não gastar outro job de agente só para repetir trabalho provado.
- A execução registra estado no SQLite externo, e o coordenador usa status/report antes de retomar.
- Mudança material exige inspect/dry-run e reconciliação explícita com motivo, conduzida pelo
  coordenador dentro do escopo. Não editar o banco nem transformar falha em sucesso.
- Interrupções técnicas são tratadas pelo coordenador enquanto houver ação autorizada útil.
  Consolidar pedidos ao usuário: credencial ou permissão realmente ausente, escolha de produto
  que muda escopo/custo, ação destrutiva não autorizada e aceite da publicação da revisão final.

Autonomia não significa que o processo continue depois que a sessão/host parar. O Forge mantém
estado recuperável. Nenhum daemon/serviço foi ativado por este planejamento; foreground é o padrão.

## Primeiro bloco: corrigir a confiabilidade antes de chamar novos agentes

O job nativo de FORGE-020 terminou com turn.completed, porém o redator substituiu uma linha JSON
inteira e o adaptador bloqueou o resultado estruturado. O bloqueio foi preservado; a validação
separada de comandos/gates passou. Não repetir esse job cegamente.

O coordenador corrige o bootstrap mínimo na sessão local, com fixture sanitizada e gate externo
executado pelo Forge como comando. Conservar eventos terminais, IDs e erros enquanto valores
sensíveis são redigidos. Construir um binário candidato e testá-lo antes de usá-lo nos jobs nativos
seguintes; o binário instalado 0.3.0 não muda por recompilar a fonte. Registrar prova verificável
do bootstrap no diretório operacional. Esta passagem não requer publicação ou decisão do usuário.

## Contratos, manifests e avaliadores

O diretório operacional escolhido é externo a todos os worktrees graváveis e contém:

```text
program.json                 índice do programa
schedule.json                ordem dos marcos
coordinator.py               preflight/status/execução sequencial
releases/implementation.yaml candidato completo para inspeção do DAG
releases/milestone-*.yaml    sete candidatos por marco
suites/NNN.py                 suites independentes por spec
evaluators/                  runners fixados por SHA-256
inspections/                 inspect/dry-run e previsões
state/                       SQLite separado por marco
logs/                        evidências e diagnósticos
```

Os arquivos de produto estão versionáveis no worktree; paths locais, estado e avaliadores
permanecem externos. Os manifests são gerados a partir do programa e validados com o Forge real.
Não usar o master e os manifests por marco simultaneamente: são visões alternativas do mesmo plano.
O modo recomendado é por marco, para checkpoints e retomadas menores. Dependências entre marcos
são verificadas pelo coordenador; dentro de cada manifest, pelo engine.

**Limite atual do plano:** as suites novas são blueprints independentes, ainda não materializadas.
Não são testes existentes, não dão sucesso e não autorizam gastar chamadas de agente. Antes de
cada marco, o coordenador escreve os testes de comportamento fora do worktree, fixa os hashes,
regenera os candidatos e roda inspect/dry-run. Essa preparação é trabalho técnico autônomo do
coordenador, não uma etapa de aprovação do usuário. O preflight bloqueia antes de chamar o modelo
quando faltar suite, bootstrap ou checkpoint anterior. Dry-run limpo prova o contrato, não a feature.

A primeira suite precisa cobrir o protocolo real; as demais cobrem os cenários nomeados em cada
spec. Testes de produto podem ser escritos pelo implementador; avaliadores de aceite não podem.
A suite deve ter casos positivos e negativos, falhar no comportamento incorreto e testar o recurso
observável. Somente teste listado, texto de agente ou JSON afirmando sucesso não constitui aceite.

## Decisões iniciais delegadas

- Núcleo Rust preservado e manifest v1 compatível; campos novos aditivos e versionados.
- Primeiro piloto de UI: web local React/TypeScript/Vite com ponte Rust e identidade Illumitech.
  Preferir login gerenciado do cliente; não copiar tokens para frontend. Registrar compatibilidade
  do App Server e validar capacidades da versão instalada antes de prometer paridade.
- Empacotamento desktop é avaliado em 0.9 por tamanho, segurança e recuperação; não bloqueia
  o chat local inicial. Revisar a escolha com evidência técnica sem parar por preferência cosmética.
- Cliques de aprovação do cliente e aceite da revisão pelo Forge são contratos distintos.
- Economia medida por entrega aceita; tempo é uma faixa com incerteza, sem converter tokens
  diretamente em segundos ou saldo de assinatura. Não gerar chamadas só para fabricar estimativas.
- ai-memory e integrações externas permanecem opcionais; skills especializadas complementam o motor.

## Política de custo e efeitos externos

Actions continua desligada. Não executar workflow_dispatch, push ou merge para descobrir se
um ajuste funciona. Validar localmente; quando for autorizado um push, preservar [skip ci] e
conferir os gatilhos. Não contornar proteções da branch nem registrar checks falsos.

Nenhum commit, push, merge, pacote no crates.io, mudança de visibilidade, instalação global,
hook ou serviço de produção faz parte dos jobs de implementação. Dependências locais necessárias
podem ser adicionadas com lock, inventário/licenças e justificativa. O piloto GitHub terá alvo e
contrato próprios antes de qualquer mutação; o coordenador prepara a prévia antes de pedir escopo
que ainda não tenha sido autorizado. Não enviar mensagens a terceiros.

Não há aprovação humana artificial entre cada tarefa ou marco. A aprovação final se associa à
revisão concreta, notas e hashes. Publicação 1.0.0, crates.io e abertura pública do GitHub são
operações separadas após essa revisão, nunca efeitos de uma tarefa concluída.

## Aceitação 1.0 e limites

- Recuperação após crash, órfãos, reconciliação, atualização e corrupção de cache comprovada.
- Tokens: consumo, qualidade, falhas, retrabalho e latência comparados em experimentos controlados.
- Tempo: erro e cobertura medidos com amostras posteriores à calibração; sem meta inventada.
- UI: conta real, sessão nova/retomada, recusa, interrupção, origem local e isolamento de projetos.
- macOS/Linux e cada plataforma anunciada estável exigem evidência nativa. Se Windows não tiver
  piloto disponível, mantê-lo experimental e declarar o limite; não parar as outras frentes.
- Claude continua experimental e fora do caminho crítico. Benchmarks pagos continuam fora de escopo.
- Relatório final mostra resultado comprovado, pendente e bloqueado separadamente.
