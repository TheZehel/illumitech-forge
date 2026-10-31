# FORGE-031 — Revisão independente da implementação

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Dependências: FORGE-021, FORGE-028, FORGE-030.

## Intenção

Implementar esta frente do programa até 1.0 sem exigir confirmações para escolhas técnicas rotineiras.
O contrato executável limita ações; prosa, checkboxes e telemetria não comprovam conclusão.

## Aceites

- F031-AC1: Papéis distintos para implementação e revisão; revisor somente leitura por padrão.
- F031-AC2: Achados citam arquivos/revisões/evidências; concordância textual não substitui testes.
- F031-AC3: Rodadas corretivas limitadas e custeadas; nenhum requisito de outro modelo, Claude ou API paga.

## Limites

Memória opcional, modelo e permissões preservados, sem bypass ou troca para API paga.
CI desativada neste programa. Nenhum hook, serviço de sistema, publicação, merge ou alteração
de visibilidade é autorizado por esta spec. Dependências locais do produto podem ser adicionadas
com versão/lock/licenças e justificativa; preservar projetos e trabalhos existentes.

## Contrato público implementado

Veja [M07 operating contracts](../../docs/m07-operating-contracts.md) e as regressões
locais em `crates/forge-cli/tests/m07.rs` e `apps/forge-ui/tests/m07.spec.ts`.
Resultados locais não substituem o ledger de aceite do coordenador.
