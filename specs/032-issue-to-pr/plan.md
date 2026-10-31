# Plano FORGE-032

Módulos prováveis: GitHub adapter; importing; publication; journals; UI.

1. Definir adaptador GitHub opt-in e mapeamento rastreável.
2. Integrar importação e draft PR com journals e políticas existentes.
3. Executar piloto em repo descartável autorizado e fixtures para erro/retry/remoto alterado; sem Actions implícita.

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

`issue.rs` importa a demanda sem autoridade; `Manifest.source` valida identidade/proveniência.
`publication.rs` conserva o journal existente e consulta o provedor antes/depois de criação,
bloqueando incerteza/divergência e Actions off. Engine e UI exibem rastreabilidade/prévia.
Regressões estão em `tests/m08.rs` e `apps/forge-ui/tests/m08.spec.ts`. O piloto real permanece
separadamente autorizado; nenhuma mutação remota é necessária para validar a implementação local.
