# Plano FORGE-024

Módulos prováveis: project; importing; spec; CLI; templates.

1. Definir CLI e contratos de templates de projeto, preservando manifest v1.
2. Implementar geração idempotente e preview; manter alterações locais e worktrees existentes.
3. Validar projeto novo e existente, repetição de setup, conflitos, permissões e caminhos com espaços.

## Validação

Fixtures e avaliadores ficam fora de worktrees graváveis. Cada suite externa é preparada e
fixada pelo coordenador antes da execução do marco; fixture ausente bloqueia, nunca aprova.
Checks direcionados durante implementação; fmt, Clippy sem warnings, workspace tests e docs
no checkpoint integrado. UI exige build/typecheck e browser real; recursos remotos exigem piloto
declarado. Mocks são regressões, não substitutos da aceitação real.

Mudança material de contrato, avaliador ou worktree invalida evidências; atualizar candidatos,
rodar inspect/dry-run e reconciliar com motivo antes de retomar. Não pedir ao usuário decisões
de biblioteca/organização rotineiras que o contrato já permite.
