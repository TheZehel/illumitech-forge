# Primeira execução / First run

A versão publicada é **1.6.0**. O pacote binário fornecido é macOS ARM64; não há binário
Linux x86_64 nesta release. Windows/PowerShell e Claude continuam experimentais. Os testes
em VM Linux não equivalem a um aceite de conta nativa. Veja a [matriz de evidências](v1-candidate.md).

## Instalação publicada

Cargo é público e requer Rust 1.94.0. Git é necessário para os projetos e o motor.

```sh
cargo install illumitech-forge-cli --version 1.6.0 --locked
forge --version
# Se outra cópia preceder o Cargo no PATH:
"${CARGO_HOME:-$HOME/.cargo}/bin/forge" --version
```

Os dois comandos nativos, `forge` e `illumitech-forge`, são instalados. Se já utiliza outro
produto chamado `forge`, preserve-o: `cargo install illumitech-forge-cli --version 1.6.0 --locked --bin illumitech-forge`.
Confirme o executável selecionado com `command -v forge` (PowerShell: `Get-Command forge -All`).
Uma cópia em outro diretório pode continuar precedendo o Cargo no PATH. Não remova arquivos
desconhecidos para corrigir isso; verifique origem e versão da instalação desejada.

O [arquivo macOS ARM64](https://github.com/TheZehel/illumitech-forge/releases/tag/v1.6.0) dispensa
Rust. O GitHub é público desde 02/11/2026. Baixe o `.tar.gz` e
`SHA256SUMS` da mesma release e verifique antes de extrair:

```sh
grep ' illumitech-forge-v1.6.0-aarch64-apple-darwin.tar.gz$' SHA256SUMS | shasum -a 256 -c -
tar -xzf illumitech-forge-v1.6.0-aarch64-apple-darwin.tar.gz
./forge --version
```

## Conversa e Workbench

Dentro de um projeto Git existente, com o cliente Codex instalado e autenticado:

```sh
forge
# Ou interface web local:
forge workbench --root . --port 0
```

O Forge preserva conta, modelo e permissões do cliente. O endereço web emitido contém uma
capacidade local: mantenha-o privado. O servidor fica em primeiro plano e atende loopback.
O estado é externo e isolado por raiz do projeto. Python, Node e ai-memory não são necessários
para abrir essas interfaces. Não é necessário executar `init` antes de conversar.

Conversa livre é somente leitura. Uma implementação exige manifest revisado e permissões
declaradas; uma mensagem ou aprovação de ferramenta não aprova gates Forge.

## Demonstracao sem agente

A demonstração nativa da 1.6.0 usa o próprio executável instalado como comando e avaliador.
Verifique antes se está executando a versão correta. Os comandos abaixo usam a instalação
padrão do Cargo; para o pacote extraído, ajuste `FORGE_BIN` para o caminho de `forge`.

```sh
FORGE_BIN="${CARGO_HOME:-$HOME/.cargo}/bin/forge"
"$FORGE_BIN" --version
FORGE_DEMO="$(mktemp -d)/example"
"$FORGE_BIN" demo create "$FORGE_DEMO"
"$FORGE_BIN" --state-dir "$FORGE_DEMO/state" release inspect "$FORGE_DEMO/release.yaml"
"$FORGE_BIN" --state-dir "$FORGE_DEMO/state" release run "$FORGE_DEMO/release.yaml" --dry-run
"$FORGE_BIN" --state-dir "$FORGE_DEMO/state" release run "$FORGE_DEMO/release.yaml"
"$FORGE_BIN" --state-dir "$FORGE_DEMO/state" report "$FORGE_DEMO/release.yaml"
```

A demonstração precisa somente de Git,
sem Python, agente, conta ou memória. Criar prepara um diretório novo; não executa a release.
Inspect/dry-run não criam estado. O resultado da execução deve ser `passed`: `app/hello.txt`
contém `Hello Forge` e `app/preserved.txt` mantém seu conteúdo original.

O manifest, o avaliador nativo e o SQLite ficam fora de `app/`. O hash do executável é fixado
como entrada do gate. Depois de trocar o binário, crie outro exemplo em um diretório novo;
não retire a verificação de hash. Diretórios existentes e artefatos modificados são preservados.
Uma falha de preparação deixa o novo diretório para inspeção, sem apagá-lo automaticamente.

Na 1.6.0, `forge installation doctor` inspeciona os dois nomes sem executar
candidatos; informa ordem, seleção divergente e limites da inspeção. Veja [detalhes](native-command.md).

## Memória opcional

Na 1.6.0, `init` prepara apenas `.forge/README.md` por padrão e preserva qualquer
`.ai-memory.toml` existente. Para criar roteamento deliberadamente:

```sh
forge init . --memory-workspace my-team
```

O parâmetro não instala hooks, não migra memória e não sobrescreve roteamento diferente.
Use letras, números, `_` e `-` no workspace. Na versão anterior 1.1.0, `init` criava
roteamento para `illumitech`; a 1.1.1 corrigiu esse comportamento, preservado na 1.6.0. Configurações existentes
continuam preservadas. Para conversar, execute `forge` diretamente, sem inicialização obrigatória.

Memória requerida só bloqueia releases que declarem `memory.required: true`. Publicação,
serviços e instalação global são ações explícitas; não acontecem ao terminar uma tarefa.

## English

Release 1.6.0: install with Cargo, verify which executable PATH selects, and run `forge`
inside an existing Git project with the native client authenticated separately. No `init`,
Python, Node or ai-memory setup is required for chat. The macOS ARM64 download and GitHub
repository are public, as is the Cargo package. Windows and Claude remain experimental.

Native `demo create`, both-name PATH diagnostics and opt-in memory initialization are included
in 1.6.0. The demo uses the real engine and a hash-pinned external native evaluator, makes
no model calls and never overwrites an existing demo directory. Status/report return 2
when evidence is stale; the historical JSON snapshot remains available for diagnosis.
