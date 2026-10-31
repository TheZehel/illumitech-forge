<p align="center"><img src="docs/assets/logo.webp" width="108" alt="Logo Illumitech"></p>

# Illumitech Forge

**Do contrato à evidência.** Organize uma demanda em specs e tarefas, execute com o cliente
nativo e acompanhe resultados verificáveis no CLI ou no Workbench Illumitech.

**Illumitech Forge 1.1.0.** Comando nativo `forge`, conversa formatada e experiência de terminal.
Compilar esta árvore não altera seu binário instalado. Consulte as
[notas da entrega](docs/release-v1.1.0.md) para artefatos, validação e limites de suporte.

[English](README.en.md) · [Começar](docs/v1-candidate.md) · [Contratos](docs/contracts.md) · [Roadmap](docs/roadmap-v1.md)

![Uma peça laranja atravessa checkpoints até uma estrutura verde-petróleo sobre fundo grafite](docs/assets/cover.webp)

A entrega **1.1.0** acrescenta o comando Rust nativo `forge` e uma experiência de terminal
com identidade ASCII, Markdown, ajuda, menu de comandos e histórico pesquisável.
As notas e a cobertura ficam em [release 1.1.0](docs/release-v1.1.0.md).

## O que está implementado

- Estimativas locais de trabalho ativo, com faixa, confiança, histórico e comparação com o realizado.
- Medição de tokens, contexto seletivo e limites de tempo, tentativas e concorrência.
- Workbench local: conta gerenciada do Codex, conversas, streaming, decisões, retomada e evidências.
- Criação guiada, proposta estruturada de specs/DAG, templates versionados e deltas de escopo.
- Controles de execução e aceite humano por revisão; cache opt-in com invalidação conservadora.
- Tarefas paralelas em worktrees existentes, um escritor por raiz, e revisão nativa independente.
- Issue selecionada → candidato revisável → execução → draft PR sob autorização explícita.
- Diagnóstico de PATH, atualização local verificada, snapshots SQLite consistentes e restauração.

Claude permanece experimental e fora do aceite nativo desta candidata. A conversa livre usa
somente leitura; a implementação usa um manifest revisado. A resposta do chat não aprova gates.

## Experimentar localmente

Requer Rust 1.94.0 e Git para compilar; Git e Python 3 para os exemplos. O Workbench usa o
Codex CLI autenticado separadamente (protocolo de referência 0.166.0, conta ChatGPT gerenciada).
Não há fallback automático para API paga ou mudança silenciosa de modelo.

```sh
cargo build --release --locked
./target/release/illumitech-forge --version
./target/release/illumitech-forge --json contracts
FORGE_OPS="$(mktemp -d)"
./target/release/illumitech-forge --state-dir "$FORGE_OPS/state" workbench --root "$PWD" --port 0
```

A conversa também possui uma [interface interativa no terminal](docs/terminal-chat.md),
disponível neste candidato com `chat --root CAMINHO_DO_PROJETO` e `--state-dir` externo.


Abra a URL local emitida e mantenha-a privada. O servidor é foreground e atende somente em
loopback. Os assets da UI vêm no binário; Node não é necessário para usá-lo. Para desenvolver
a UI, use Node 24 LTS e `npm ci --prefix apps/forge-ui --ignore-scripts`.

Para um exemplo sem inferência, crie outro diretório externo e execute:

```sh
FORGE_DEMO="$(mktemp -d)/demo"
python3 scripts/demo.py "$FORGE_DEMO"
./target/release/illumitech-forge --state-dir "$FORGE_DEMO/state" release inspect "$FORGE_DEMO/release.yaml"
./target/release/illumitech-forge --state-dir "$FORGE_DEMO/state" release run "$FORGE_DEMO/release.yaml" --dry-run
./target/release/illumitech-forge --state-dir "$FORGE_DEMO/state" release run "$FORGE_DEMO/release.yaml"
./target/release/illumitech-forge --state-dir "$FORGE_DEMO/state" report "$FORGE_DEMO/release.yaml"
```

A instalação da versão 1.1.0, após sua publicação, será feita com
`cargo install illumitech-forge-cli --version 1.1.0 --locked`.

O candidato também fornece o executável Rust nativo **`forge`**. Basta executar `forge`
para abrir a conversa no projeto atual; fora de um projeto, escolha uma pasta na interface.
Veja [entrada nativa e instalação](docs/native-command.md).
A instalação global é uma ação explícita; o código não troca seu PATH automaticamente.

## Ciclo de trabalho

Pedido → specs → proposta validada → inspect/dry-run → execução → gates → revisão humana,
quando declarada. Propostas são candidatas sem autorização de executar ou publicar. Mantenha
manifests, avaliadores fixados por hash e SQLite fora do diretório gravável pelo agente.

```mermaid
flowchart LR
  S[Specs e ADRs] --> M[Manifest revisado]
  M --> F[Forge]
  F --> C[Cliente nativo ou comando]
  C --> W[Worktrees]
  W --> G[Gates e artefatos externos]
  G --> D[SQLite e eventos]
  A[ai-memory opcional via MCP] --> F
```

Specs/ADRs expressam intenção; manifests são contratos; SQLite guarda o estado operacional;
Git guarda código. ai-memory oferece continuidade por MCP e **não é obrigatório**. O modo
`required: true` só deve ser escolhido quando o projeto realmente exigir memória verificável.
Skills especializadas complementam o motor. O Loop legado continua disponível para migração gradual.

## Limites e qualidade

As métricas cobrem as chamadas registradas pelo Forge, não toda a conta. Cache de entrada é
parte da entrada; ausência de uso permanece desconhecida. Uma chamada nativa pode fazer várias
requisições ao provedor. Hooks, MCPs e outras sessões preservam suas configurações e podem ter
atividade fora desse registro. Não há promessa de economia percentual, preço ou prazo.

O cache exige observação conservadora dos insumos. Árvores com symlinks em `node_modules` ou
builds além do limite observado provocam miss e nova validação. Não foi demonstrada economia
pelo cache na própria execução deste programa. [Medições e limites](docs/v1-candidate.md).

macOS tem pilotos nativos em checkpoints anteriores; o aceite da revisão final fica no ledger
externo. Linux ARM64 tem baseline local em VM/contêiner; o sandbox nativo da conta foi bloqueado
nesse laboratório. Windows/PowerShell permanece experimental, sem piloto nativo. Não se presume
paridade entre mocks, execução em VM e conta nativa. [Matriz](docs/v1-candidate.md#plataformas).

CI, publicação, merge, mudança de visibilidade, instalação global e ativação de serviços são
operações separadas. `budget.actions: off` bloqueia escritas remotas que possam ativar CI;
`[skip ci]` não é garantia de ausência de Actions. O programa de implementação usa checks locais.

## Documentação e créditos

[Workbench](docs/workbench.md) · [Economia](docs/token-economy.md) · [Duração](docs/duration-estimates.md) ·
[Templates/orçamentos/cache](docs/m06-operating-contracts.md) · [Paralelismo/revisão](docs/m07-operating-contracts.md) ·
[Issue/PR](docs/m08-operating-contracts.md) · [Atualização/recuperação](docs/m09-operating-contracts.md) ·
[Serviços opt-in](docs/user-services.md) · [Distribuição](docs/public-distribution.md)

Copyright Illumitech. Licença [MIT](LICENSE). Avisos de dependências em
[THIRD_PARTY_NOTICES](THIRD_PARTY_NOTICES), inventário em [docs/dependencies.json](docs/dependencies.json)
e origem dos ativos em [provenance.json](docs/assets/provenance.json). Commits e publicações usam
identidade humana configurada, sem atribuição de autoria a agentes.
