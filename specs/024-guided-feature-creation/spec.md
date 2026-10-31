# FORGE-024 — Criação guiada de funcionalidades

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Dependências: FORGE-021.

## Intenção

Implementar esta frente do programa até 1.0 sem exigir confirmações para escolhas técnicas rotineiras.
O contrato executável limita ações; prosa, checkboxes e telemetria não comprovam conclusão.

## Aceites

- F024-AC1: feature new prepara spec/plan/tasks e candidato de manifest com IDs permanentes e diretórios operacionais externos.
- F024-AC2: Detectar convenções e stack sem sobrescrever arquivos existentes nem executar instalação implicitamente.
- F024-AC3: Gerar dependências e aceites verificáveis; campos desconhecidos exigem esclarecimento somente quando impedem uma decisão segura.

## Limites

Memória opcional, modelo e permissões preservados, sem bypass ou troca para API paga.
CI desativada neste programa. Nenhum hook, serviço de sistema, publicação, merge ou alteração
de visibilidade é autorizado por esta spec. Dependências locais do produto podem ser adicionadas
com versão/lock/licenças e justificativa; preservar projetos e trabalhos existentes.

## Implementação local

Contratos, comandos, limites e cobertura: [workbench](../../docs/workbench.md).
O ledger externo continua responsável pelo aceite da revisão.
