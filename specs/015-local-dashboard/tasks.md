# Tarefas FORGE-015

Aceite externo pendente; implementação e validações locais descritas em `docs/dashboard.md`.
Os checkboxes ficam pendentes até evidência externa da revisão, conforme os aceites do spec.

- [ ] F015-T01: Consultas read-only e paginação; evidência: F015-AC1/AC2; fixtures ausente, corrompida e com eventos concorrentes.
- [ ] F015-T02: Servidor, rotas e segurança; evidência: F015-AC3/AC5; testes HTTP de XSS, Host/Origin, métodos e preservação. <!-- forge: {"dependencies":["F015-T01"]} -->
- [ ] F015-T03: Interface e guia de uso; evidência: F015-AC4; revisão visual/teclado em 360px e 1280px mais gates locais. <!-- forge: {"dependencies":["F015-T02"]} -->
