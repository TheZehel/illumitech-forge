# Plano FORGE-023

Módulos prováveis: apps/forge-ui (novo); ponte Rust; App Server; dashboard; contratos.

1. Validar protocolo por spike e fixtures; registrar ADR de compatibilidade sem pedir escolha técnica rotineira ao usuário.
2. Implementar MVP web local com React/TypeScript/Vite e ponte Rust em origem local autenticada; manter empacotamento desktop para 0.9.
3. Validar fluxos com browser real e Codex autenticado; falhas, recusa e reconexão não duplicam ações.

## Validação

Fixtures e avaliadores ficam fora de worktrees graváveis. Cada suite externa é preparada e
fixada pelo coordenador antes da execução do marco; fixture ausente bloqueia, nunca aprova.
Checks direcionados durante implementação; fmt, Clippy sem warnings, workspace tests e docs
no checkpoint integrado. UI exige build/typecheck e browser real; recursos remotos exigem piloto
declarado. Mocks são regressões, não substitutos da aceitação real.

Mudança material de contrato, avaliador ou worktree invalida evidências; atualizar candidatos,
rodar inspect/dry-run e reconciliar com motivo antes de retomar. Não pedir ao usuário decisões
de biblioteca/organização rotineiras que o contrato já permite.
