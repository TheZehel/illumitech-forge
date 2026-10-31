# FORGE-034 — Estabilização e aceitação integrada

Status: implementação local candidata 1.0.0; aceite da revisão final pertence ao ledger externo. Veja docs/v1-candidate.md.
Dependências: FORGE-020, FORGE-021, FORGE-022, FORGE-023, FORGE-024, FORGE-025, FORGE-026, FORGE-027, FORGE-028, FORGE-029, FORGE-030, FORGE-031, FORGE-032, FORGE-033.

## Intenção

Implementar esta frente do programa até 1.0 sem exigir confirmações para escolhas técnicas rotineiras.
O contrato executável limita ações; prosa, checkboxes e telemetria não comprovam conclusão.

## Aceites

- F034-AC1: Contratos CLI/JSON/manifest com política de compatibilidade e documentação PT/EN.
- F034-AC2: Pilotos de uso real por projeto: criar demanda, planejar, estimar, executar, revisar e recuperar.
- F034-AC3: Economia e tempo avaliados com baseline e dados não usados na calibração; qualidade e falhas reportadas sem omissão.
- F034-AC4: Cada plataforma anunciada estável exige piloto nativo; Windows continua experimental se não houver ambiente.
- F034-AC5: Pacotes, licenças, segredos, backups/restore e inventário verificados; revisão final disponível para aceite humano.
- F034-AC6: Publicação, visibilidade do GitHub e crates.io são operações separadas, após autorização da revisão concreta.

## Limites

Memória opcional, modelo e permissões preservados, sem bypass ou troca para API paga.
CI desativada neste programa. Nenhum hook, serviço de sistema, publicação, merge ou alteração
de visibilidade é autorizado por esta spec. Dependências locais do produto podem ser adicionadas
com versão/lock/licenças e justificativa; preservar projetos e trabalhos existentes.
