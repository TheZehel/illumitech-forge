# Tarefas FORGE-018

Aceite final pendente: exige gates externos da revisão e ciclos nativos.
Implementação local: `crates/forge-cli/src/service.rs`, testes de fixtures em
`scripts/test_service_fixtures.py` e ciclo opt-in em `scripts/test_service_native.py`.
Os checkboxes abaixo não substituem nem antecipam aceitação pelo avaliador.

- [ ] F018-T01: Plano e renderizadores nativos; evidência: F018-AC1; fixtures plist/unit/XML com espaços e Unicode.
- [ ] F018-T02: Aplicação e remoção preservadoras; evidência: F018-AC2/AC4; idempotência, rollback e arquivo modificado. <!-- forge: {"dependencies":["F018-T01"]} -->
- [ ] F018-T03: Controle e status; evidência: F018-AC3; lock, pausa e heartbeat sem mutação na consulta. <!-- forge: {"dependencies":["F018-T02"]} -->
- [ ] F018-T04: Aceite por plataforma; evidência: F018-AC5; ciclo real descartável e limites documentados, sem ativar o host. <!-- forge: {"dependencies":["F018-T03"]} -->
