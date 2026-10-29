# Plano FORGE-016

Estado: tooling offline e runbook implementados; integração nativa e aceitação operacional pendentes.

Implementação: `scripts/memory_migration.py`, testes `scripts/test_memory_migration.py` e
[runbook](../../docs/native-memory-migration.md). O launcher nativo permanece sob controle do
operador externo: não há flags presumidas, sandbox automático ou execução de transição em produção.
Recibos declarativos de isolamento/autorização exigem revisão com evidência independente.

1. Criar tooling de inventário/plano e verificador offline de backups, com caminhos explícitos e recusas.
2. Implementar ensaio por fases com recibos externos; cada fase revalida pré-condições antes de aplicar.
3. Documentar comandos suportados pelo binário ai-memory fixado, sem inventar health endpoints;
   falha de handshake aborta antes de troca. Desativar jobs externos no ambiente isolado.
4. Usar fixtures sintéticas para IDs, conteúdos e escopos; testar backup/restore e rollback compatível.
5. Preparar checklist revisável de cutover e rollback. Produção, Docker ativo e configuração dos
   clientes permanecem intocados nesta tarefa; ensaio real exige ambiente explicitamente selecionado.
A migração anterior 1.17.3 → 3.2.9 é histórica e não comprova migração nativa.

## Validação e entrega
Executar `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` e `cargo doc --workspace --no-deps`. Testes novos devem exercitar
comportamentos positivos e recusas; não desabilitar testes existentes para obter sucesso.
Registrar revisão, comando, resultado e hashes de artefatos em recibos externos sanitizados.
Gates nativos, autenticados ou externos não executados permanecem explicitamente pendentes.
