# FORGE-030 — Paralelismo em worktrees isolados

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Dependências: FORGE-028, FORGE-029.

## Intenção

Implementar esta frente do programa até 1.0 sem exigir confirmações para escolhas técnicas rotineiras.
O contrato executável limita ações; prosa, checkboxes e telemetria não comprovam conclusão.

## Aceites

- F030-AC1: Tarefas independentes em worktrees distintos com um escritor por worktree e concorrência limitada.
- F030-AC2: Dependências respeitadas e orçamento compartilhado; integração final avaliada sem resolver conflitos destrutivamente.
- F030-AC3: Cancelar encerra árvores de processos; estimador passa a considerar o caminho crítico real e recursos limitantes.

## Limites

Memória opcional, modelo e permissões preservados, sem bypass ou troca para API paga.
CI desativada neste programa. Nenhum hook, serviço de sistema, publicação, merge ou alteração
de visibilidade é autorizado por esta spec. Dependências locais do produto podem ser adicionadas
com versão/lock/licenças e justificativa; preservar projetos e trabalhos existentes.

## Contrato público implementado

Veja [M07 operating contracts](../../docs/m07-operating-contracts.md) e as regressões
locais em `crates/forge-cli/tests/m07.rs` e `apps/forge-ui/tests/m07.spec.ts`.
Resultados locais não substituem o ledger de aceite do coordenador.
