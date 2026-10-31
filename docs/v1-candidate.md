# Candidata local 1.0.0

Esta candidata reúne FORGE-020–034. Não foi publicada e não substitui a instalação 0.3.0.
Os resultados históricos abaixo pertencem aos respectivos checkpoints. Aceite da fonte final,
pilotos e hashes são produzidos pelo operador fora dos worktrees graváveis; não são inferidos
de checkboxes, deste documento ou de mensagens do agente.

## Usar o CLI e a UI

Compile com `cargo build --release --locked`; consulte `--help` e `--json contracts`.
Execute o Workbench com `--state-dir EXTERNAL_STATE workbench --root GIT_ROOT --port 0`.
Abra a URL local privada impressa. Para acompanhar um contrato já revisado, acrescente
`--manifest EXTERNAL_MANIFEST`. O processo é foreground; a UI e seus assets são embarcados.
A conta gerenciada e o modelo vêm do cliente Codex autenticado, sem substituição por API paga.

A conversa livre é somente leitura. A seleção da raiz delimita as rotas HTTP e a propriedade
das sessões; a visibilidade real dos arquivos e MCPs do cliente nativo depende da configuração
herdada. Não é uma promessa de confinamento de toda leitura ao repositório. Não há concessão
automática de permissões. Recarregar/reconectar não repete turnos com resultado desconhecido.

Consultas HTTP de leitura da UI têm prazo de cinco segundos. Uma leitura travada apresenta
erro e conserva os controles globais já autorizados. Operações de escrita não são repetidas
automaticamente quando a resposta é desconhecida. Logs sanitizam campos de credenciais
também em fragmentos JSON inválidos ou truncados, preservando os envelopes válidos.

Interromper uma conversa solicita a interrupção do turno nativo. Terminais nativos em segundo
plano podem continuar; esse botão não equivale ao cancelamento de workers de uma release.
Na retomada explícita, a UI preserva os estados finais observados no histórico nativo, sem
iniciar inferência nem inventar conclusão para estados desconhecidos.

O fluxo guiado cria spec/plan/tasks e um manifest candidato externo, sem sobrescrever edições.
`plan propose` usa o cliente nativo em modo somente leitura; `plan validate` é determinístico.
Propostas, comandos sugeridos e seus aceites precisam de revisão antes da implementação.
Instalar ferramentas, ampliar permissões e publicar são decisões explícitas do contrato.

## Operar uma demanda

1. Use `feature new --root GIT_ROOT --id 001-feature --title TITLE --ops-root EXTERNAL_OPS`
   para prévia; acrescente `--apply` para criar os arquivos revisados. `templates list/plan/apply`
   oferece receitas versionadas de Rust, Node e genéricas, sem instalar a stack.
2. Gere/revise a proposta com `plan propose`; valide IDs, DAG e delta de escopo. Fixe os
   avaliadores externos por SHA-256, critérios, modelos autorizados e permissões do executor.
3. Execute `release inspect MANIFEST` e `release run MANIFEST --dry-run` com o mesmo estado.
   A prévia inclui comandos, dependências, permissões, contexto, orçamento e faixa de duração.
4. Use `release run MANIFEST`, `status`, `report`, `pause`, `resume` e `cancel`. A UI selecionada
   oferece os controles locais equivalentes. Serial pause é cooperativo; paralelismo interrompe
   todos os grupos ativos. Não reutilize um PID antigo como prova de propriedade.
5. `review plan/run` mantém a revisão separada da implementação. Achados referenciam arquivo,
   linha, hash e revisão; divergência não fabrica aceite. Corrija e revalide no escopo autorizado.
6. `release approve` registra somente o aceite humano declarado para a fingerprint efetivamente
   revisada. Mudanças materiais exigem reconciliação; eventos antigos permanecem históricos.

Um manifest pode selecionar worktrees existentes e `execution.max_parallel`, limitado por
`budget.max_concurrency`. O scheduler respeita dependências, ondas e um escritor por raiz.
Não cria branches nem resolve conflitos por sobrescrita ou merge destrutivo. Gates finais
reavaliam a fonte integrada. Native calls com teto compartilhado de tokens podem serializar
até que o consumo observado permita nova admissão.

## Economia, orçamento e duração

```yaml
context:
  profile: bounded
  budget_tokens: 8192
budget:
  actions: off
  max_attempts: 8
  max_active_seconds: 1800
  max_concurrency: 2
```

Use `context plan MANIFEST --task TASK --budget-tokens N` para revisar as fontes sem inferência.
O orçamento da montagem de prompt é uma heurística local, não um teto de todos os tokens do
cliente. Instruções obrigatórias e aceites são preservados; conteúdo desconhecido não vira zero.
Configurações globais, ferramentas, hooks e MCPs podem acrescentar contexto/atividade fora do
registro do Forge. Uma invocação de CLI ou turno App Server não equivale a uma requisição HTTP
única ao provedor. Cache de entrada é subconjunto da entrada, não tokens adicionais.

Limites de tempo, tentativas e concorrência são aplicados na admissão/execução. Tokens dependem
dos contadores nativos; um teto duro dentro da inferência só existe quando o cliente o suporta.
Consumo ausente bloqueia novas chamadas sob uma política que precise desse dado. Preço/saldo de
assinatura não são inferidos; um teto monetário sem custo observável bloqueia a execução.

O cache de gates é opt-in e depende de identidade do avaliador, fonte, ambiente, toolchain,
plataforma e diagnóstico verificável. Humano e execução nativa não são cacheados como gates.
A observação da árvore é conservadora: symlinks em dependências e diretórios de build além dos
limites provocam miss. Este programa não demonstrou reuso da sua própria suíte de qualidade.
Não exclua insumos só para forçar hit. Veja os contratos M06/M07 para os limites completos.

A faixa de duração representa trabalho ativo, com confiança e premissas. Aprovação, pausa e
esperas externas não recebem uma ETA inventada. O relatório preserva a previsão inicial e mede
erro/cobertura. Os pequenos ensaios abaixo não estabelecem precisão nem economia generalizável.

| Ensaio histórico | Observação |
|---|---|
| Tokens, par 1: broad → bounded | 86.243 → 77.223 tokens; 38,999 → 30,950 s; ambos aceitos |
| Tokens, par 2: bounded → broad | 77.311 → 86.237 tokens; 33,788 → 30,252 s; ambos aceitos |
| Duração, sem histórico | previsão 9–400 s; observado 1,94 s; fora da faixa |
| Duração, 3 calibrações + 1 amostra posterior | previsão 2,1655–43,1655 s; observado 0,671 s; fora da faixa |

Os pares nativos foram exploratórios em checkpoints anteriores, com ordem invertida, cache do
provedor sem controle e operações distintas entre pares. Houve inversão do resultado de tempo.
Os totais incluem entrada/cache conforme o cliente reporta e saída; cache não é somado duas
vezes. Um job nativo do M06 também excedeu o limite superior inicialmente previsto. Não se
ajustam parâmetros a uma amostra e depois se apresenta a mesma amostra como validação independente.
A fonte final recebe seus próprios checks e observações, separados desses ensaios históricos.

## Issue, PR e CI

`issue import --repository OWNER/REPO --number N --root GIT_ROOT --ops-root EXTERNAL_OPS`
consulta só a issue selecionada; `--source-json FILE` usa snapshot offline e registra essa origem.
Prévia é o padrão; `--apply` cria candidatos, sem executar instruções remotas. Conteúdo de issue
não configura shell, modelos, permissões ou autorização de publicação. O import é create-only.

Inspect/report mostram origem, hash de conteúdo, specs, tarefas e evidências atuais. Draft PR
reusa o journal existente e faz readback antes de repetir criação. Mudança remota, ambiguidade,
PR fechado/mesclado ou desfecho desconhecido exige reconciliação, sem duplicar ou sobrescrever PR.
O hostname GitHub é fixado nos comandos de consulta e mutação; `GH_HOST` não redireciona o alvo.

`budget.actions: off` bloqueia escritas remotas que possam acionar workflows. Para permitir
esses efeitos, o contrato exige autorização de CI separada e `actions: on`; isso não comprova
saldo ou cota disponível. A UI não executa publicação. `[skip ci]` não é garantia de ausência
de Actions. Os ensaios de publicação deste programa usam adaptadores locais, sem push/PR real.
Merge, release, crates.io e visibilidade continuam operações explícitas e separadas.

## Atualização e recuperação

`installation doctor` ajuda a identificar PATH e binários concorrentes, sem trocar sua instalação.
`installation update --artifact FILE --sha256 SHA --prefix PREFIX --dry-run` valida a prévia;
`--apply` altera apenas o prefixo explícito, mantém backup/recibo e recusa downgrade silencioso,
symlinks inseguros, produto desconhecido e checksum incorreto. Hash fornecido pelo chamador não
é assinatura nem prova independente de autoria do distribuidor. Probes têm limites.

Use `--state-dir STATE state snapshot --output SNAPSHOT` após parar escritores. A cópia usa
backup consistente do SQLite e verifica banco, hashes e artefatos. Daemon/Workbench ativos
impedem um snapshot que não possa estabelecer consistência. Logs e bancos adicionais suportados
são preservados; um banco desconhecido exige suporte explícito.

`state restore --snapshot SNAPSHOT --target-state-dir FRESH --dry-run` não escreve; `--apply`
restaura só em destino novo. Corrupção e esquema incompatível falham sem sobrescrever o estado
existente. Preserve o binário correspondente ao backup. Não abra estado migrado com executável
anterior; o piso de leitura do container de snapshot não garante recuperação operacional de
features posteriores. Depois da restauração, confira fonte, processos e evidências antes de
retomar. Não opere o estado original e sua restauração como escritores concorrentes.

## Plataformas

| Superfície | Evidência disponível antes do aceite final |
|---|---|
| macOS ARM64 | Pilotos nativos anteriores de planejamento e UI; checks locais dos marcos; fonte final requer seus próprios pilotos |
| Linux ARM64 | M06 em VM Ubuntu/contêiner Debian: fmt, Clippy, testes, docs, build e quatro fixtures externas passaram; revalidação final separada |
| Conta nativa Linux | CLI 0.166.0 inicia; sandbox bloqueado no contêiner e na VM do laboratório; sem login/copiar credenciais ou afrouxar permissões |
| Windows/PowerShell | Experimental; piloto nativo indisponível; sem alegar suporte estável a partir de mocks ou resultados anteriores |
| Claude | Experimental e adiado por decisão de escopo |

Não foi instalada uma aplicação desktop nativa. O ADR M09 escolhe browser local com assets
embarcados, evitando outro runtime/updater. Instalação automática de serviços e ativação global
não fazem parte desta rodada. Os comandos de serviço continuam opt-in.

## Contratos e distribuição

`--json contracts` descreve versões, compatibilidade, erros e plataformas. Outputs JSON têm
`schema_version: 1` e `tool_version`; campos novos de saída são aditivos. Inputs rejeitam campos
não suportados. Manifest/planning/snapshot/recibo têm contratos próprios. Frames RPC nativos
mantêm seu protocolo. Help/version e erros do parser Clap ficam fora do envelope de comandos.
Resultado bloqueado, falha, cancelamento ou espera de aceite retorna código 2, sem autorizar retry.

Os pacotes são verificados localmente por extração/build com patches temporários dos crates
irmãos. Isso não é resolução no registry nem upload. O script `scripts/candidate.py` sela a árvore
atual e artefatos selecionados em destino novo com hashes; o selo não concede aceite humano.
Artefatos, inventário, revisão e relatório externo precisam ser avaliados antes de publicar.
Nenhum commit, push, merge, instalação global, Actions ou publicação é efeito deste programa.

## Cobertura por spec

| Specs | Implementação e cobertura local |
|---|---|
| 020–022 | duration.rs/usage/context; lifecycle/economy; protocolo, histórico, isolamento e falhas |
| 023–025 | native/workbench/guided/planning; testes de browser, fixtures e propostas; piloto real separado |
| 026–029 | templates/budget/validation_cache; m06 e controles browser; cache inválido e orçamento ausente |
| 030–031 | parallel/review; m07; barreiras, conflitos, cancelamento SIGINT/SIGTERM, revisão maliciosa e logs |
| 032 | issue/publication; m08/parity; rastreabilidade, CI, timeout/readback e host fixo; remoto real não executado |
| 033 | installation/snapshots/state; m09; checksum, downgrade, writers, corrupção, restore e assets standalone |
| 034 | contracts-v1/compatibility, m10, scripts de distribuição/licenças/privacidade e gates externos finais |

Os blueprints iniciais de suites por spec não foram executados como prova. A execução usou
jobs agrupados, com manifests/avaliadores externos e ledgers separados. Históricos bloqueados
ou interrompidos continuam preservados; a recuperação não promove uma tentativa antiga.

## Dependência de build corrigida

O lock fixa esbuild 0.28.1, compatível com o intervalo aceito pelo Vite 7.3.7, para remover
GHSA-g7r4-m6w7-qqqr. O aviso cobre o servidor de desenvolvimento do esbuild no Windows; o
Workbench distribuído serve assets pelo servidor Rust, sem esse servidor de desenvolvimento.
O override, os binários opcionais e os avisos acompanham o lock. O build e a suíte da UI
são revalidados após a alteração. Fonte: [aviso do mantenedor](https://github.com/evanw/esbuild/security/advisories/GHSA-g7r4-m6w7-qqqr).
