# FORGE-021 — Confiabilidade do protocolo e continuidade

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Dependências: Base 0.3.0; estimador local preservado.

## Intenção

Implementar esta frente do programa até 1.0 sem exigir confirmações para escolhas técnicas rotineiras.
O contrato executável limita ações; prosa, checkboxes e telemetria não comprovam conclusão.

## Aceites

- F021-AC1: Preservar JSON/JSONL válido ao redigir strings sensíveis; não descartar type, IDs, erros ou eventos terminais.
- F021-AC2: Reproduzir o log bloqueado no job FORGE-020 com fixture sanitizada; execução 0 sem turn.completed continua bloqueada.
- F021-AC3: Separar falhas transitórias de permissão, contrato, autenticação e evidência; retry limitado, sem repetição cega.
- F021-AC4: Manter journals completos para recuperação e relatórios paginados/limitados; nenhuma aprovação é transferida.

## Limites

Memória opcional, modelo e permissões preservados, sem bypass ou troca para API paga.
CI desativada neste programa. Nenhum hook, serviço de sistema, publicação, merge ou alteração
de visibilidade é autorizado por esta spec. Dependências locais do produto podem ser adicionadas
com versão/lock/licenças e justificativa; preservar projetos e trabalhos existentes.
