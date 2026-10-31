# Plano FORGE-028

Módulos prováveis: policy; scheduler; clients; UI; adapters GitHub.

1. Definir política local e contrato de orçamento com dados ausentes explícitos.
2. Integrar orçamento à seleção de tarefas, clientes e UI sem alterar modelos silenciosamente.
3. Validar orçamento esgotado, saldo inacessível, recuperação e ausência de disparos implícitos de CI.

## Validação

Fixtures e avaliadores ficam fora de worktrees graváveis. Cada suite externa é preparada e
fixada pelo coordenador antes da execução do marco; fixture ausente bloqueia, nunca aprova.
Checks direcionados durante implementação; fmt, Clippy sem warnings, workspace tests e docs
no checkpoint integrado. UI exige build/typecheck e browser real; recursos remotos exigem piloto
declarado. Mocks são regressões, não substitutos da aceitação real.

Mudança material de contrato, avaliador ou worktree invalida evidências; atualizar candidatos,
rodar inspect/dry-run e reconciliar com motivo antes de retomar. Não pedir ao usuário decisões
de biblioteca/organização rotineiras que o contrato já permite.
