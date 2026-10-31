# FORGE-025 — Planejamento assistido e prévia de execução

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Dependências: FORGE-022, FORGE-024.

## Intenção

Implementar esta frente do programa até 1.0 sem exigir confirmações para escolhas técnicas rotineiras.
O contrato executável limita ações; prosa, checkboxes e telemetria não comprovam conclusão.

## Aceites

- F025-AC1: pedido → specs → DAG → comandos/gates → previsão de tempo e consumo antes de executar.
- F025-AC2: Cliente nativo produz proposta validada estruturalmente; não inferir conclusão de texto nem habilitar publicação.
- F025-AC3: Mudança de escopo mostra delta de plano e atualiza estimativas; decisão técnica rotineira segue política e ADR.

## Limites

Memória opcional, modelo e permissões preservados, sem bypass ou troca para API paga.
CI desativada neste programa. Nenhum hook, serviço de sistema, publicação, merge ou alteração
de visibilidade é autorizado por esta spec. Dependências locais do produto podem ser adicionadas
com versão/lock/licenças e justificativa; preservar projetos e trabalhos existentes.

## Implementação local

Contratos, comandos, limites e cobertura: [workbench](../../docs/workbench.md).
O ledger externo continua responsável pelo aceite da revisão.
