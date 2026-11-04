# 1.5.0 — correção guiada e autonomia local

Base: 1.4.1. O registro `release/changes.json` determina incremento minor.
Distribuição e validação nativa desta entrega: macOS ARM64. Linux x86_64 adiado pelo operador e acompanhado em https://github.com/TheZehel/illumitech-forge/issues/6.

- `/bug` e “Planejar correção de bug” geram uma proposta pelo planejador nativo existente, com verificador externo visível e aprovação do digest antes da reprodução.
- Preparação recuperável preserva o journal e confere fonte, contrato e checkpoints; resultado incerto bloqueia replay.
- Tentativas de planejamento têm registros locais, inclusive interrupções sem candidato. `workflow export` mostra uma prévia por allowlist; `--output` grava explicitamente e `--human-minutes` identifica tempo manual. `workflow attempts --root PROJETO --ops-root OPERACOES_DE_PLANEJAMENTO` lista também falhas sem workflow; `workflow export ID_DA_TENTATIVA` usa a mesma prévia por allowlist. Tentativa e workflow compartilham pseudônimo para não contar o mesmo planejamento duas vezes. Espera é medida somente entre parada persistida e retomada explícita; espera nativa ou anterior à execução permanece desconhecida.
- Kit de piloto de 14 dias disponível em `docs/pilot-1.5.0.md`; nenhum convite é enviado pelo produto.

Aceite da revisão final, testes nativos, pacotes e checksums pertencem ao ledger externo. A suíte local registrou 278 testes Rust e 27 testes de navegador aprovados; instalação, atualização isolada, terminal, recuperação de crash e correção controlada com Codex autenticado foram validados em macOS ARM64. Esses resultados não concedem aceite Linux. Windows/Claude experimentais; Linux em VM não equivale a aceite de conta nativa. Sem Actions, hospedagem, telemetria ou instalação global.
