# FORGE-028 — Orçamento de execução e CI

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Dependências: FORGE-022, FORGE-025.

## Intenção

Implementar esta frente do programa até 1.0 sem exigir confirmações para escolhas técnicas rotineiras.
O contrato executável limita ações; prosa, checkboxes e telemetria não comprovam conclusão.

## Aceites

- F028-AC1: Limites por tarefa/release de tempo, tentativas e concorrência; tokens e dinheiro somente onde mensuráveis.
- F028-AC2: CI desligada por padrão neste programa; preview do custo/saldo disponível ou desconhecido, sem alegar garantia falsa.
- F028-AC3: Interromper antes de novas chamadas que ultrapassem política; teto de inferência só se o cliente suportar.

## Limites

Memória opcional, modelo e permissões preservados, sem bypass ou troca para API paga.
CI desativada neste programa. Nenhum hook, serviço de sistema, publicação, merge ou alteração
de visibilidade é autorizado por esta spec. Dependências locais do produto podem ser adicionadas
com versão/lock/licenças e justificativa; preservar projetos e trabalhos existentes.
