# FORGE-026 — Templates de trabalho versionados

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Dependências: FORGE-024, FORGE-025.

## Intenção

Implementar esta frente do programa até 1.0 sem exigir confirmações para escolhas técnicas rotineiras.
O contrato executável limita ações; prosa, checkboxes e telemetria não comprovam conclusão.

## Aceites

- F026-AC1: Receitas Rust, Node/TypeScript e genérica para bug, feature, revisão, dependências e release.
- F026-AC2: Origem, revisão, versão, escopo e compatibilidade explícitos; nada baixado de main é executado automaticamente.
- F026-AC3: Personalização por projeto mantém stack e convenções; atualização mostra diff e preserva mudanças locais.

## Limites

Memória opcional, modelo e permissões preservados, sem bypass ou troca para API paga.
CI desativada neste programa. Nenhum hook, serviço de sistema, publicação, merge ou alteração
de visibilidade é autorizado por esta spec. Dependências locais do produto podem ser adicionadas
com versão/lock/licenças e justificativa; preservar projetos e trabalhos existentes.
