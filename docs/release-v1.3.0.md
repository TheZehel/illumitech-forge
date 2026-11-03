# Illumitech Forge 1.3.0 — atividades estruturadas

Candidata preparada localmente. A publicação no GitHub e no crates.io depende do aceite
da revisão final; estes arquivos não constituem recibo de publicação.

## Novidades

- Workbench com chamadas MCP, comandos e alterações em cartões recolhidos por padrão:
  **Mostrar detalhes / Ocultar detalhes**, argumentos, resultado, erro, diretório,
  código de saída e diffs por arquivo. Atualizações preservam os cartões abertos.
- Terminal com inspeção individual por **F2** ou `/activities`: ↑/↓ selecionam,
  Enter/Espaço alternam detalhes, ←/→ recolhem/expandem e PgUp/PgDn percorrem a saída.
  Cada arquivo abre seu próprio diff. Esc retorna ao rascunho preservado.
- Ctrl+O mantém a expansão geral na conversa e alterna a seleção no painel. Ajuda
  compacta e reposicionamento da seleção ao redimensionar ou reabrir o painel.
- Compactação preserva indicadores de falha, IDs, caminhos e metadados de renomeação
  independentemente do orçamento de conteúdo. Resultado MCP com `isError: true`
  continua como falha mesmo quando o cliente informa transporte `completed`.
- Limite de 64 KiB por evento exibido, até 32 arquivos retidos com total original
  explícito. Conteúdo parcial tem aviso; escapes JSON não podem eliminar metadados.
  Metadados que sozinhos ultrapassem o limite geram aviso explícito.

## Instalar após a publicação

```sh
cargo install illumitech-forge-cli --version 1.3.0 --locked
forge --version
```

Até a publicação, a versão estável disponível continua sendo a 1.2.0. A compilação
local não altera o binário instalado nem outros produtos chamados `forge`.

## Validação e cobertura

A implementação desta candidata passou por 252 testes Rust e 26 de navegador,
typecheck, formatação, Clippy sem warnings, build e documentação. A reprodução do
P2 usa três textos MCP de 30 mil caracteres e diffs extensos pela ponte real, com
fixtures de protocolo no Workbench e em PTY. Esses pilotos não invocam modelos e
não substituem aceite com conta nativa ou uma consulta real a serviços externos.

Os gates finais de distribuição, a varredura, os hashes e a revisão avaliada são
registrados no artefato externo `RELEASE-VERIFICATION.json`. Resultados anteriores
não aprovam automaticamente uma fonte alterada.

## Distribuição e limites

Preparação dos quatro crates MIT, fontes e pacote macOS ARM64 com os dois nomes
nativos (`forge` e `illumitech-forge`), licenças, avisos e checksums. Instalação,
upload, merge, tag, GitHub release e deployment do site são ações separadas.

ai-memory permanece opcional; configuração, autenticação e modelo do cliente são
preservados. Chat ou uma ferramenta concluída não aprovam gates. Claude e Windows/
PowerShell continuam experimentais; esta rodada não acrescenta aceite nativo Linux.
O site mantém a identidade visual vigente e continua como prévia local. Actions
permanecem desativadas nesta preparação. Não há promessa de economia de tokens.
