# Plano FORGE-026

Módulos prováveis: catalog; project; spec; recipes; documentação.

1. Definir catálogo e seleção de receitas sem instalação implícita.
2. Implementar renderização/upgrade com preview, merge conservador e proveniência.
3. Validar todas as receitas em projetos descartáveis, setup repetido e conflitos.

## Validação

Fixtures e avaliadores ficam fora de worktrees graváveis. Cada suite externa é preparada e
fixada pelo coordenador antes da execução do marco; fixture ausente bloqueia, nunca aprova.
Checks direcionados durante implementação; fmt, Clippy sem warnings, workspace tests e docs
no checkpoint integrado. UI exige build/typecheck e browser real; recursos remotos exigem piloto
declarado. Mocks são regressões, não substitutos da aceitação real.

Mudança material de contrato, avaliador ou worktree invalida evidências; atualizar candidatos,
rodar inspect/dry-run e reconciliar com motivo antes de retomar. Não pedir ao usuário decisões
de biblioteca/organização rotineiras que o contrato já permite.
