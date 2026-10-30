# Plano FORGE-019

Estado: implementação local FORGE-EXP-OBS; aceite externo vinculado à revisão e ingestão real pendentes.
Ver [contrato implementado e limites](../../docs/observability.md).

1. Introduzir interface sink entre domínio, adaptadores e forge-cli com implementação noop padrão e adaptador HTTP isolado.
2. Instrumentar fases reais da engine/daemon/publicação, preservando resultado original e saída CLI.
3. Usar IDs e hierarquia consistentes; SQLite continua autoridade operacional e recebe apenas diagnóstico
   sanitizado de degradação. Traces não são evidência para aprovadores nem source of truth.
4. Implementar transporte limitado e redaction antes do enqueue; confirmar protocolo atual nas fontes
   oficiais do provider durante implementação, sem chamada real ou instalação de SDK nesta tarefa.
5. Testar com fake server local determinístico e revisar matriz Loop → Forge; não transportar limitações
   do fail-open do context manager Python para o runtime Rust.

## Validação e entrega
Executar `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` e `cargo doc --workspace --no-deps`. Testes novos devem exercitar
comportamentos positivos e recusas; não desabilitar testes existentes para obter sucesso.
Registrar revisão, comando, resultado e hashes de artefatos em recibos externos sanitizados.
Gates nativos, autenticados ou externos não executados permanecem explicitamente pendentes.
