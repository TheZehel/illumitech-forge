# FORGE-029 — Reaproveitamento seguro de validações

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Dependências: FORGE-021, FORGE-028.

## Intenção

Implementar esta frente do programa até 1.0 sem exigir confirmações para escolhas técnicas rotineiras.
O contrato executável limita ações; prosa, checkboxes e telemetria não comprovam conclusão.

## Aceites

- F029-AC1: Reutilizar somente evidências cujos insumos relevantes, avaliador, dependências e ambiente sejam comprovadamente iguais.
- F029-AC2: Registrar cache hit/miss e motivo; memória/contexto resumido não é evidência.
- F029-AC3: Aceite humano não se transfere para revisão nova; parâmetros não observáveis exigem invalidação conservadora.

## Limites

Memória opcional, modelo e permissões preservados, sem bypass ou troca para API paga.
CI desativada neste programa. Nenhum hook, serviço de sistema, publicação, merge ou alteração
de visibilidade é autorizado por esta spec. Dependências locais do produto podem ser adicionadas
com versão/lock/licenças e justificativa; preservar projetos e trabalhos existentes.
