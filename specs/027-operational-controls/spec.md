# FORGE-027 — Controle operacional na UI

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Dependências: FORGE-023, FORGE-025.

## Intenção

Implementar esta frente do programa até 1.0 sem exigir confirmações para escolhas técnicas rotineiras.
O contrato executável limita ações; prosa, checkboxes e telemetria não comprovam conclusão.

## Aceites

- F027-AC1: Iniciar, pausar, cancelar e retomar pela UI usando as mesmas regras da CLI.
- F027-AC2: Aceite humano mostra revisão, diff e fingerprint; recusa e revisão desatualizada permanecem bloqueadas.
- F027-AC3: Ações autenticadas por origem local, auditadas e idempotentes; uma UI desconectada não concede permissão.

## Limites

Memória opcional, modelo e permissões preservados, sem bypass ou troca para API paga.
CI desativada neste programa. Nenhum hook, serviço de sistema, publicação, merge ou alteração
de visibilidade é autorizado por esta spec. Dependências locais do produto podem ser adicionadas
com versão/lock/licenças e justificativa; preservar projetos e trabalhos existentes.
