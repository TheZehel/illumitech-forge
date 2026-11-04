# Illumitech Forge 1.4.1 — leitura contínua no terminal

## Correções

- Mouse/trackpad e PgUp/PgDn percorrem respostas que ultrapassam a viewport.
- Subir mantém a posição de leitura enquanto novas saídas chegam. Ctrl+End retoma o acompanhamento do fim; Ctrl+Home volta ao início.
- Redimensionar preserva a linha lógica; a contagem total do histórico visível não é truncada para u16.
- Cabeçalho, rodapé e editor compactos ampliam a conversa, preservando projeto/modelo, avisos e decisões nativas. No cenário sintético 90×24, o conteúdo legível passou de 10 para 17 linhas.
- Rascunho e permissões preservados. A captura do mouse e o modo terminal são restaurados ao sair.

## Atualizar após publicação

```sh
cargo install illumitech-forge-cli --version 1.4.1 --locked
forge --version
forge doctor
```

Reabra sessões antigas após atualizar. Se outro binário aparecer, revise o PATH pelo doctor. A navegação usa a área rolável do Forge; não muda a retenção do histórico pelo cliente nativo nem o scrollback externo do emulador. Use Shift ao selecionar texto em terminais que reservam a seleção ao aplicativo.

## Validação e distribuição

Checks e hashes finais estão em RELEASE-VERIFICATION.json. Testes de renderização e PTY usam protocolo sintético, sem inferência real; não constituem novo aceite de conta nativa. Fonte e quatro crates MIT, binários macOS ARM64 e checksums vinculados à revisão avaliada. Metadados de versão não são recibos de upload.

Workbench preservado; site somente como prévia local. ai-memory continua opcional. Claude e Windows/PowerShell continuam experimentais; sem novo aceite nativo Linux. Nenhum workflow de Actions ou instalação global é ativado por esta release.
