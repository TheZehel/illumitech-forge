# Plano FORGE-025

Módulos prováveis: planejador; manifest; importing; duration; UI.

1. Implementar contrato de proposta e validação determinística, com timeout e limites.
2. Integrar cliente nativo e estimador existente; não delegar autoridade a conteúdo de repositório ou memória.
3. Testar plano inválido, ambiguidade relevante, ciclo, referência inexistente e revisão do plano antes de executar.

## Validação

Fixtures e avaliadores ficam fora de worktrees graváveis. Cada suite externa é preparada e
fixada pelo coordenador antes da execução do marco; fixture ausente bloqueia, nunca aprova.
Checks direcionados durante implementação; fmt, Clippy sem warnings, workspace tests e docs
no checkpoint integrado. UI exige build/typecheck e browser real; recursos remotos exigem piloto
declarado. Mocks são regressões, não substitutos da aceitação real.

Mudança material de contrato, avaliador ou worktree invalida evidências; atualizar candidatos,
rodar inspect/dry-run e reconciliar com motivo antes de retomar. Não pedir ao usuário decisões
de biblioteca/organização rotineiras que o contrato já permite.
