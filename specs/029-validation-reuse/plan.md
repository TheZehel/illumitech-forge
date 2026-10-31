# Plano FORGE-029

Módulos prováveis: evidence; revision; engine; store; report.

1. Definir chaves, níveis de confiança e retenção de evidências.
2. Implementar cache local verificável sem pular gates obrigatórios ou perder journals.
3. Validar mudanças em código, avaliador, manifest, permissões, toolchain, cache corrompido e artefatos ignorados.

## Validação

Fixtures e avaliadores ficam fora de worktrees graváveis. Cada suite externa é preparada e
fixada pelo coordenador antes da execução do marco; fixture ausente bloqueia, nunca aprova.
Checks direcionados durante implementação; fmt, Clippy sem warnings, workspace tests e docs
no checkpoint integrado. UI exige build/typecheck e browser real; recursos remotos exigem piloto
declarado. Mocks são regressões, não substitutos da aceitação real.

Mudança material de contrato, avaliador ou worktree invalida evidências; atualizar candidatos,
rodar inspect/dry-run e reconciliar com motivo antes de retomar. Não pedir ao usuário decisões
de biblioteca/organização rotineiras que o contrato já permite.
