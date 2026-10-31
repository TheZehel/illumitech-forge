# Plano FORGE-030

Módulos prováveis: scheduler; locks; worktrees; process; duration; integration.

1. Definir scheduler, ownership e política de integração de resultados.
2. Implementar concorrência limitada e recuperação de processos/locks órfãos.
3. Validar conflitos, falhas parciais, cancelamento, crash e equivalência de qualidade com execução serial.

## Validação

Fixtures e avaliadores ficam fora de worktrees graváveis. Cada suite externa é preparada e
fixada pelo coordenador antes da execução do marco; fixture ausente bloqueia, nunca aprova.
Checks direcionados durante implementação; fmt, Clippy sem warnings, workspace tests e docs
no checkpoint integrado. UI exige build/typecheck e browser real; recursos remotos exigem piloto
declarado. Mocks são regressões, não substitutos da aceitação real.

Mudança material de contrato, avaliador ou worktree invalida evidências; atualizar candidatos,
rodar inspect/dry-run e reconciliar com motivo antes de retomar. Não pedir ao usuário decisões
de biblioteca/organização rotineiras que o contrato já permite.

## Implementação local

O contrato está em `docs/m07-operating-contracts.md`. Scheduler/ownership em
`parallel.rs`, revisão em `review.rs`, gates/controles em `engine.rs` e
`validation_cache.rs`; a UI usa operações tipadas e captura raízes no startup.
Checkpoints locais e limitações constam do relatório M07; aceite segue externo.
