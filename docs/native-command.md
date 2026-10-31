# Entrada nativa `forge`

Este candidato inclui dois executáveis Rust no mesmo pacote: `forge` e `illumitech-forge`.
Compartilham a implementação e os contratos de execução. Não há alias, script intermediário,
Python ou caminhos da máquina do desenvolvedor no comando. A entrega 1.1.0 inclui esta alteração; sua publicação é uma etapa explícita
separada da validação local.

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
A partir de uma cópia deste candidato:

```sh
cargo install --path crates/forge-cli --locked
```

Os arquivos de release incluem `forge`/`forge.exe`, `illumitech-forge`/`illumitech-forge.exe`
e checksums de ambos. Não sobrescreva um comando `forge` de outro produto; verifique
`command -v forge` (ou `Get-Command forge`). Se desejar apenas o nome longo, Cargo permite
`--bin illumitech-forge`. O atualizador legado de artefato único continua gerenciando o
executável de nome longo; atualizações dos dois executáveis são feitas via Cargo ou pelo
pacote de release completo.
