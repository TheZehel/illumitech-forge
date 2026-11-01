# Entrada nativa `forge`

A versão 1.1.1 inclui dois executáveis Rust no mesmo pacote: `forge` e `illumitech-forge`.
Compartilham a implementação e os contratos de execução. Não há alias, script intermediário,
Python ou caminhos da máquina do desenvolvedor no comando. A entrega publicada 1.1.1 inclui esta alteração; a entrega 1.1.1 acrescenta a demonstração e diagnósticos abaixo.

```sh
forge                         # Abre o chat do projeto atual
forge --root /caminho/projeto  # Abre outro projeto
forge --help
forge --version
forge release inspect /caminho/externo/release.yaml
```

Sem subcomando, `forge` abre a interface de terminal. Em uma subpasta encontra a raiz Git.
Fora de um projeto, lista os projetos Git filhos imediatos: ↑/↓ escolhem, Enter abre e Esc
cancela. Nenhum cliente ou banco operacional é iniciado antes da escolha. Não cria Git,
altera projetos, executa releases ou concede permissões automaticamente.

Chat e workbench usam estado externo separado por raiz canônica, em
`$XDG_STATE_HOME/illumitech-forge/workbench/HASH` ou
`$HOME/.local/state/illumitech-forge/workbench/HASH` em macOS/Linux. Em Windows, a base é
`%LOCALAPPDATA%\IllumitechForge\State`; o suporte operacional continua experimental.
`--state-dir` explícito prevalece e mantém as verificações de segurança existentes.
Os demais comandos mantêm seus defaults e contratos, incluindo estado explícito para snapshots.
Abertura com `illumitech-forge chat` também recebe o estado automático; `illumitech-forge`
sem argumentos continua mostrando ajuda. Autenticação e modelo continuam nativos.

## Instalação e distribuição

[Cargo instala os binários do pacote](https://doc.rust-lang.org/cargo/commands/cargo-install.html).
Para instalar a versão publicada:

```sh
cargo install illumitech-forge-cli --version 1.1.1 --locked
```

Os arquivos de release incluem `forge`/`forge.exe`, `illumitech-forge`/`illumitech-forge.exe`
e checksums de ambos. Não sobrescreva um comando `forge` de outro produto; verifique
`command -v forge` (ou `Get-Command forge`). Se desejar apenas o nome longo, Cargo permite
`--bin illumitech-forge`. O atualizador legado de artefato único continua gerenciando o
executável de nome longo; atualizações dos dois executáveis são feitas via Cargo ou pelo
pacote de release completo.

## Diagnóstico desde 1.1.1

`forge installation doctor` agora inspeciona os nomes `forge` e `illumitech-forge`, aponta a
primeira opção elegível no PATH e informa quando é um caminho diferente do executável em
execução. `path_diagnostics` é um campo aditivo do JSON schema 1; os campos legados permanecem.

O diagnóstico nunca executa candidatos nem presume suas versões. A versão informada no topo
é apenas a do processo atual. Inspeciona no máximo 128 entradas; declara truncamento, e não
inspeciona aliases, funções, cache do shell ou ACLs. Não altera perfis nem remove concorrentes.

Se atualizar pelo Cargo e continuar abrindo outra cópia, compare `command -v forge` com
`~/.cargo/bin/forge installation doctor` (em PowerShell: `Get-Command forge -All`). Confira
a origem antes de executar outros candidatos. Preserve produtos diferentes que usem o mesmo nome.
