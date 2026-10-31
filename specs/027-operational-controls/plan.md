# Plano FORGE-027

Módulos prováveis: UI; API local; controls; approvals; state.

1. Expor API local tipada e segura de controle; manter dashboard legado somente leitura.
2. Implementar fluxos de aprovação e execução com journal e reconciliação.
3. Validar concorrência, pedidos duplicados, origem indevida, cancelamento e revisão alterada.

## Validação

Fixtures e avaliadores ficam fora de worktrees graváveis. Cada suite externa é preparada e
fixada pelo coordenador antes da execução do marco; fixture ausente bloqueia, nunca aprova.
Checks direcionados durante implementação; fmt, Clippy sem warnings, workspace tests e docs
no checkpoint integrado. UI exige build/typecheck e browser real; recursos remotos exigem piloto
declarado. Mocks são regressões, não substitutos da aceitação real.

Mudança material de contrato, avaliador ou worktree invalida evidências; atualizar candidatos,
rodar inspect/dry-run e reconciliar com motivo antes de retomar. Não pedir ao usuário decisões
de biblioteca/organização rotineiras que o contrato já permite.
