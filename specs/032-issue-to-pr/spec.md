# FORGE-032 — Issue → execução → PR

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Dependências: FORGE-025, FORGE-026, FORGE-027, FORGE-031.

## Intenção

Implementar esta frente do programa até 1.0 sem exigir confirmações para escolhas técnicas rotineiras.
O contrato executável limita ações; prosa, checkboxes e telemetria não comprovam conclusão.

## Aceites

- F032-AC1: Importar demanda selecionada e manter rastreabilidade de issue/spec/task/commit/PR.
- F032-AC2: Preview de ações remotas e permissões; commit/push/draft dependem do contrato explícito do projeto.
- F032-AC3: Falha/retry consulta estado remoto e não duplica PR; autoria humana e nenhuma identidade de agente em labels de publicação.
- F032-AC4: Merge e release não ocorrem implicitamente por uma tarefa ou comentário remoto.

## Limites

Memória opcional, modelo e permissões preservados, sem bypass ou troca para API paga.
CI desativada neste programa. Nenhum hook, serviço de sistema, publicação, merge ou alteração
de visibilidade é autorizado por esta spec. Dependências locais do produto podem ser adicionadas
com versão/lock/licenças e justificativa; preservar projetos e trabalhos existentes.

## Contrato implementado

[Operação M08](../../docs/m08-operating-contracts.md) descreve importação selecionada,
proveniência versionada, rastreabilidade e publicação com journal/readback. Testes locais
e fixtures de provedor não concedem aceite nem autorização para piloto remoto.
