# FORGE-052 — Preparação da release 1.3.0

Consolidar FORGE-051 e a correção P2 de compactação em candidata revisável. Publicação
e aceite da revisão final permanecem separados da preparação local.

## Aceites
- AC1: versões 1.3.0 coerentes entre workspace, dependências internas exatas, locks e pacotes; documentação distingue candidata de versão publicada.
- AC2: varredura local de segredos, SAST com escopo declarado, dependências OSV/npm e licenças com resultados e limitações explícitos. Achados reais são corrigidos ou bloqueiam a entrega; fixtures não são vulnerabilidades reais.
- AC3: evidências vinculadas à fonte final, com testes Rust, Python e UI, regressões P2 no Workbench/PTY, build otimizado e verificações de primeira execução. Reutilização somente com prova dos inputs idênticos.
- AC4: fontes, quatro crates extraídos/recompilados offline sob lock, pacote macOS ARM64 e checksums prontos para revisão, sem credenciais/dados internos e sem incluir estado operacional. Evidências e avaliadores fora do worktree.
- AC5: sem uso de Actions, upload, deployment, tag remota, merge ou instalação global. Commit local de candidata pode identificar a fonte preparada; publicação requer autorização da revisão.
