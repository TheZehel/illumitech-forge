# FORGE-033 — Instalação, atualização e recuperação

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Dependências: FORGE-021, FORGE-027, FORGE-029.

## Intenção

Implementar esta frente do programa até 1.0 sem exigir confirmações para escolhas técnicas rotineiras.
O contrato executável limita ações; prosa, checkboxes e telemetria não comprovam conclusão.

## Aceites

- F033-AC1: Diagnosticar PATH e binários concorrentes; atualizar com origem/checksums verificados e preview.
- F033-AC2: Snapshot consistente e restore testado antes de migrar estado; nunca abrir banco migrado com binário antigo.
- F033-AC3: Validar macOS/Linux/Windows declarados; serviços por usuário sem privilégios elevados ou login automático implícito.
- F033-AC4: Empacotamento desktop da UI decidido por ADR de tamanho, segurança e recuperação; sem paridade presumida.

## Limites

Memória opcional, modelo e permissões preservados, sem bypass ou troca para API paga.
CI desativada neste programa. Nenhum hook, serviço de sistema, publicação, merge ou alteração
de visibilidade é autorizado por esta spec. Dependências locais do produto podem ser adicionadas
com versão/lock/licenças e justificativa; preservar projetos e trabalhos existentes.

## Local implementation candidate

`installation.rs`, `snapshots.rs`, domain maintenance leases and `tests/m09.rs` implement
the selected contract. See [M09 operating contracts](../../docs/m09-operating-contracts.md)
and [ADR 0003](../../docs/adr/0003-safe-updates-and-browser-packaging.md). No incompatible
schema migration is introduced. Local regressions are not coordinator acceptance; task
checkboxes remain owned by the external ledger. Linux/Windows native pilots are pending;
Windows is experimental. No operational files, services or publication are changed.
