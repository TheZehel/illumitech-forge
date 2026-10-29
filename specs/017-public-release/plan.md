# Plano FORGE-017

Estado: preparação local implementada; aceite externo e publicação pendentes.

1. Inventariar Cargo.toml, grafo real, licenças e scripts; escolher nomes prefixados coerentes.
2. Preparar metadados e script de package/list/validação local por ordem de dependência.
3. Adicionar auditoria pública read-only de fonte, histórico e arquivos empacotados; não imprimir achados secretos.
4. Produzir runbook com gates de preparação, autorização e operação separados; nenhum token em exemplos.
5. Publisher verifica `--expected-visibility PRIVATE|PUBLIC`, padrão PRIVATE, sem alterar visibilidade.
   Ambos preservam revisão aprovada, tag remota e checksums. crates.io permanece operação separada.
6. Validar pacotes localmente; compatibilidade registry e ownership permanecem pendentes até consulta real.

## Validação e entrega
Executar `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` e `cargo doc --workspace --no-deps`. Testes novos devem exercitar
comportamentos positivos e recusas; não desabilitar testes existentes para obter sucesso.
Registrar revisão, comando, resultado e hashes de artefatos em recibos externos sanitizados.
Gates nativos, autenticados ou externos não executados permanecem explicitamente pendentes.
