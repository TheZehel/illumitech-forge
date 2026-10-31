# Plano FORGE-034

Módulos prováveis: todos os módulos; docs; testes externos; distribuição.

1. Congelar interfaces da candidata, inventariar limites e completar documentação e migrações.
2. Executar aceitação real e testes de recuperação, isolamento, consumo e duração; produzir relatório verificável.
3. Preparar notas, pacotes e hashes para aprovação final da 1.0.0; não publicar neste programa de implementação.

## Validação

Fixtures e avaliadores ficam fora de worktrees graváveis. Cada suite externa é preparada e
fixada pelo coordenador antes da execução do marco; fixture ausente bloqueia, nunca aprova.
Checks direcionados durante implementação; fmt, Clippy sem warnings, workspace tests e docs
no checkpoint integrado. UI exige build/typecheck e browser real; recursos remotos exigem piloto
declarado. Mocks são regressões, não substitutos da aceitação real.

Mudança material de contrato, avaliador ou worktree invalida evidências; atualizar candidatos,
rodar inspect/dry-run e reconciliar com motivo antes de retomar. Não pedir ao usuário decisões
de biblioteca/organização rotineiras que o contrato já permite.
