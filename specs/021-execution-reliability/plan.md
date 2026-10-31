# Plano FORGE-021

Módulos prováveis: adapters/clients e process; engine; state; integração do cliente.

1. Fixar fixtures externas de protocolo, redação e falso aceite; verificar o estimador existente no novo worktree.
2. Corrigir adaptadores e continuidade, com testes negativos e preservação de configuração.
3. Validar CLI e cliente real em repositório descartável; bootstrap deve usar o código corrigido antes dos jobs nativos seguintes.

## Validação

Fixtures e avaliadores ficam fora de worktrees graváveis. Cada suite externa é preparada e
fixada pelo coordenador antes da execução do marco; fixture ausente bloqueia, nunca aprova.
Checks direcionados durante implementação; fmt, Clippy sem warnings, workspace tests e docs
no checkpoint integrado. UI exige build/typecheck e browser real; recursos remotos exigem piloto
declarado. Mocks são regressões, não substitutos da aceitação real.

Mudança material de contrato, avaliador ou worktree invalida evidências; atualizar candidatos,
rodar inspect/dry-run e reconciliar com motivo antes de retomar. Não pedir ao usuário decisões
de biblioteca/organização rotineiras que o contrato já permite.
