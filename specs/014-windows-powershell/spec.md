# FORGE-014 — Windows e PowerShell

## Intenção e estado
Adicionar suporte operacional nativo para usuários de Windows e PowerShell.
Escopo solicitado após o aceite da v0.2.0. Planejado para a próxima entrega; a v0.2.0
permanece suportada somente em macOS ARM64 e Linux x86_64. WSL não substitui o aceite nativo.

## Plataforma inicial
Windows 11 x86_64, PowerShell 7 e binário MSVC `illumitech-forge.exe`.
Windows PowerShell 5.1 deve ser avaliado explicitamente: detectar incompatibilidade e documentar
instalação do PowerShell 7; não anunciar suporte 5.1 sem testes. ARM64 Windows fica fora do primeiro alvo.

## Aceites
- F014-AC1: build e testes nativos em runner Windows, sem dependência de Bash/WSL para usar a CLI.
- F014-AC2: ZIP com EXE, MIT, avisos, exemplos PowerShell e SHA256SUMS; instalação por usuário,
  PATH e desinstalação reversíveis, sem exigir administrador. Idempotência preserva arquivos existentes.
- F014-AC3: caminhos com espaços, Unicode, separadores, CRLF e argv/quoting funcionam; comandos
  `inspect`, dry-run, run, status, pause/resume/cancel e importação de specs têm exemplos PowerShell.
- F014-AC4: encerramento da árvore de processos, timeout, Ctrl+C, processos órfãos, SQLite e locks
  entre instâncias são comprovados nativamente. Avaliar Job Objects; não substituir cancelamento por
  encerramento apenas do PID pai. Daemon permanece opt-in, sem serviço instalado automaticamente.
- F014-AC5: cliente nativo autenticado funciona em execução real, preserva modelo/permissões e
  arquivos; gh/Git, draft PR, retomada e publicação têm piloto isolado. Sem fallback para APIs pagas.
- F014-AC6: ai-memory segue opcional. Caso configurado, MCP e escopo precisam ser verificáveis;
  não exigir Docker nem instalar hooks duplicados para usar Forge no Windows.
- F014-AC7: CI macOS/Linux continua passando; README distingue alvos comprovados e experimentais.
  Publicação Windows exige checksums, evidências reais e aceite humano da revisão.

## Limites
Não migrar projetos existentes, alterar autenticação, instalar serviços ou declarar equivalência
com PowerShell 5.1 sem validação. Commits e releases usam autoria humana, sem trailers de agente.
