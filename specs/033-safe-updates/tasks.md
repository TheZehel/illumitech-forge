# Tarefas FORGE-033

Status: backlog; marcas de conclusão dependem do ledger externo da revisão.

- [ ] F033-T01: Definir política de compatibilidade e pacote/atualização com backup e rollback. Aceites: FORGE-033. <!-- forge: {"dependencies": ["F021-T03", "F027-T03", "F029-T03"], "priority": 330} -->
- [ ] F033-T02: Implementar diagnóstico, atualização explícita e migração recuperável de estado/serviços. Aceites: FORGE-033. <!-- forge: {"dependencies": ["F033-T01"], "priority": 331} -->
- [ ] F033-T03: Testar upgrade interrompido, falha de checksum, PATH antigo, restauração e piloto Windows com ambiente disponível. Aceites: FORGE-033. <!-- forge: {"dependencies": ["F033-T02"], "priority": 332} -->

## Regressões mínimas previstas

- `update_verifies_source_and_checksums`
- `migration_failure_restores_matching_backup`
- `path_conflicts_and_interrupted_update_are_recoverable`

Nomes são contratos de cobertura a materializar em suites externas antes do marco;
não são testes existentes nem evidências de implementação.

## Local implementation candidate

`installation.rs`, `snapshots.rs`, domain maintenance leases and `tests/m09.rs` implement
the selected contract. See [M09 operating contracts](../../docs/m09-operating-contracts.md)
and [ADR 0003](../../docs/adr/0003-safe-updates-and-browser-packaging.md). No incompatible
schema migration is introduced. Local regressions are not coordinator acceptance; task
checkboxes remain owned by the external ledger. Linux/Windows native pilots are pending;
Windows is experimental. No operational files, services or publication are changed.
