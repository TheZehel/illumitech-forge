# Plano FORGE-033

Módulos prováveis: installer; migration; service; packaging; doctor; UI shell.

1. Definir política de compatibilidade e pacote/atualização com backup e rollback.
2. Implementar diagnóstico, atualização explícita e migração recuperável de estado/serviços.
3. Testar upgrade interrompido, falha de checksum, PATH antigo, restauração e piloto Windows com ambiente disponível.

## Validação

Fixtures e avaliadores ficam fora de worktrees graváveis. Cada suite externa é preparada e
fixada pelo coordenador antes da execução do marco; fixture ausente bloqueia, nunca aprova.
Checks direcionados durante implementação; fmt, Clippy sem warnings, workspace tests e docs
no checkpoint integrado. UI exige build/typecheck e browser real; recursos remotos exigem piloto
declarado. Mocks são regressões, não substitutos da aceitação real.

Mudança material de contrato, avaliador ou worktree invalida evidências; atualizar candidatos,
rodar inspect/dry-run e reconciliar com motivo antes de retomar. Não pedir ao usuário decisões
de biblioteca/organização rotineiras que o contrato já permite.

## Local implementation candidate

`installation.rs`, `snapshots.rs`, domain maintenance leases and `tests/m09.rs` implement
the selected contract. See [M09 operating contracts](../../docs/m09-operating-contracts.md)
and [ADR 0003](../../docs/adr/0003-safe-updates-and-browser-packaging.md). No incompatible
schema migration is introduced. Local regressions are not coordinator acceptance; task
checkboxes remain owned by the external ledger. Linux/Windows native pilots are pending;
Windows is experimental. No operational files, services or publication are changed.
