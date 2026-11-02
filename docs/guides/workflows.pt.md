# Do pedido à entrega verificável

Disponível na versão 1.2.0. Compilar ou atualizar uma instalação não muda a ordem do seu PATH; confira qual executável está selecionado.

## Começar

Execute `forge` dentro de um projeto Git. Escolha Executar uma tarefa no Workbench ou use `/plan PEDIDO` no terminal. Planejamento usa o cliente nativo em modo somente leitura e não executa o plano.

Revise objetivo, tarefas, arquivos previstos, comandos, regressões e permissões. Salvar o candidato cria apenas dados operacionais externos. Uma confirmação do digest autoriza esse contrato; não aceita antecipadamente seus resultados.

No terminal, `/run DIGEST` confirma o plano exibido. No Workbench, use Aprovar escopo e executar. Solicitações nativas de permissão continuam independentes.

## CLI e automação

```sh
forge workflow doctor --root .
forge workflow recipes --root .
forge workflow prepare request.json --root . --ops-root /caminho/externo/operacao
forge workflow prepare request.json --root . --ops-root /caminho/externo/operacao --apply
forge workflow start 001-correcao --root . --ops-root /caminho/externo/operacao --approve-scope DIGEST
forge workflow status 001-correcao --root . --ops-root /caminho/externo/operacao
```

O JSON request exige schema_version, id, title, request e recipe. Checks são argv revisáveis; se omitidos, o Forge sugere os scripts existentes ou verificações Cargo. Contexto limitado a 16.384 tokens estimados é padrão apenas dos novos workflows. Instruções obrigatórias não são descartadas para caber. Scripts watch devem ser substituídos por comandos de execução única durante a revisão.

## Receitas

Feature implementa um pedido delimitado. Bug exige uma regressão externa congelada: um check com before_exit não zero e arquivo em files. Referencie scripts como argumento completo `{checks}/regression.js`; `{repo}` é substituído somente como argumento completo.

Dependencies exige um check com rescan:true (OSV-Scanner ou Semgrep com saída SARIF; Semgrep exige --error) além dos testes do projeto. Organiza uma atualização selecionada: importe SARIF 2.1.0, revise o achado e acrescente rescan e testes ao contrato. Dados e sugestões do scanner nunca são comandos autorizados automaticamente. Achado importado permanece não verificado; desaparecimento do relatório sozinho não demonstra correção.

PR prepara diff, resultados de testes e descrição local. Commit, push, abertura de PR e publicação exigem outras ações explícitas.

## Aceite e recuperação

Gates externos determinam conclusão; texto do agente não comprova sucesso. human_gates preserva um aceite humano independente associado à revisão avaliada. Comandos legados e manifests existentes conservam seus contratos.

Use `/tasks`, `/tasks ID`, `/report ID` e `/tasks resume ID`. `/resume` continua retomando conversas. No Workbench, selecione uma tarefa salva para recuperar diagnóstico, evidências e controles.

Retomada não repete start. Contratos alterados exigem novo escopo. Alterações de fonte exigem revisão e reconciliação explícita pelo engine. Preparação ou baseline interrompidos antes do checkpoint de execução ficam bloqueados: inspecione o journal e arquivos antes de preparar outro workflow. Processos desconhecidos nunca são considerados concluídos.

## Memória e privacidade

ai-memory é opcional. Nenhum hook é instalado. Conta, modelo e permissões continuam gerenciados pelo cliente nativo. O site não se conecta ao seu Workbench nem coleta código, conversas ou telemetria.

Chamadas de planejamento bem-sucedidas vinculadas pelo servidor entram no journal de tokens da execução. Planejamento falho ou não vinculado e outras sessões ficam fora desse total, explicitamente.

## Scanner externo em instalações Homebrew

Verificadores congelados ficam fora de árvores Git. O Homebrew pode manter o próprio prefixo em Git; nesse caso, o executável de rescan instalado ali é recusado. Para Semgrep, selecione uma cópia revisada do ponto de entrada na sua área operacional externa, preservando o runtime já instalado:

```sh
FORGE_OPS=/caminho/externo/operacao
mkdir -p "$FORGE_OPS/scanner"
cp -p "$(command -v semgrep)" "$FORGE_OPS/scanner/semgrep"
shasum -a 256 "$(command -v semgrep)" "$FORGE_OPS/scanner/semgrep"
```

Confira os dois hashes e use o caminho absoluto da cópia como primeiro argumento do check. O Forge fixa esse arquivo no contrato; não instala nem atualiza Semgrep. A cópia do ponto de entrada continua usando o runtime Homebrew original: seu hash não atesta todas as bibliotecas do scanner. Revise a versão e prepare um novo contrato se a instalação mudar.
