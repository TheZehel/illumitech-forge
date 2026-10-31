# FORGE-023 — UI Illumitech conectada ao Codex

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Dependências: FORGE-021, FORGE-022.

## Intenção

Implementar esta frente do programa até 1.0 sem exigir confirmações para escolhas técnicas rotineiras.
O contrato executável limita ações; prosa, checkboxes e telemetria não comprovam conclusão.

## Aceites

- F023-AC1: Ponte local tipada App Server, com negociação de capacidades, versões e conta/modelo/permissões autorizados.
- F023-AC2: Projetos e sessões, streaming, retomada, interrupção, diffs e aprovações reais, com estados claros de desconexão.
- F023-AC3: Layout Illumitech com logo existente, laranja #ff7a1a, grafite e verde-petróleo; navegação por teclado e contraste.
- F023-AC4: Conversa livre e execução sob manifest são distintas; resposta do chat não equivale a gate aprovado.
- F023-AC5: Tokens e estimativa do plano usam contratos do motor; sem credenciais no frontend, acesso remoto ou importação automática de sessões.

## Limites

Memória opcional, modelo e permissões preservados, sem bypass ou troca para API paga.
CI desativada neste programa. Nenhum hook, serviço de sistema, publicação, merge ou alteração
de visibilidade é autorizado por esta spec. Dependências locais do produto podem ser adicionadas
com versão/lock/licenças e justificativa; preservar projetos e trabalhos existentes.

## Implementação local

Contratos, comandos, limites e cobertura: [workbench](../../docs/workbench.md).
O ledger externo continua responsável pelo aceite da revisão.
