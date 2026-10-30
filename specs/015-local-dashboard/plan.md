# Plano FORGE-015

Estado: implementação local do CLI e testes HTTP presentes; aceite externo da revisão pendente.
Consulte `docs/dashboard.md` para uso e limites. Checkboxes continuam dependentes de evidência externa.

1. Separar consultas limitadas de status no domínio/adaptadores, preservando schema e CLI existentes.
2. Adicionar comando e listener no forge-cli; renderizar HTML escapado e ativos embutidos, sem build remoto.
3. Definir DTOs com allowlist de campos; nenhum retorno de snapshots/logs inteiros.
4. Testar HTTP em porta efêmera e SQLite descartável externo; verificar preservação e shutdown.
5. Fazer revisão visual manual local e registrar limites; nenhuma ativação persistente.
Sem migração de schema nem nova autoridade de aprovação pela interface.

## Validação e entrega
Executar `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` e `cargo doc --workspace --no-deps`. Testes novos devem exercitar
comportamentos positivos e recusas; não desabilitar testes existentes para obter sucesso.
Registrar revisão, comando, resultado e hashes de artefatos em recibos externos sanitizados.
Gates nativos, autenticados ou externos não executados permanecem explicitamente pendentes.
