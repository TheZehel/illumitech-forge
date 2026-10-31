# FORGE-022 — Economizador de tokens

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Dependências: FORGE-021.

## Intenção

Implementar esta frente do programa até 1.0 sem exigir confirmações para escolhas técnicas rotineiras.
O contrato executável limita ações; prosa, checkboxes e telemetria não comprovam conclusão.

## Aceites

- F022-AC1: Medir entrada, cache e saída sem duplicar contadores cumulativos; usage ausente permanece desconhecido.
- F022-AC2: Contexto seletivo com fontes e recuperação sob demanda preserva instruções, aceites e isolamento por projeto.
- F022-AC3: Evitar chamadas redundantes e incluir consumo de planejamento, resumos, falhas e cancelamentos.
- F022-AC4: Comparação controlada mede tokens por entrega aceita, sucesso, retrabalho e latência; sem percentual prometido ou troca silenciosa de modelo.

## Limites

Memória opcional, modelo e permissões preservados, sem bypass ou troca para API paga.
CI desativada neste programa. Nenhum hook, serviço de sistema, publicação, merge ou alteração
de visibilidade é autorizado por esta spec. Dependências locais do produto podem ser adicionadas
com versão/lock/licenças e justificativa; preservar projetos e trabalhos existentes.
