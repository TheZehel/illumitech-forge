# ADR 0001 — Escopo da primeira entrega

Data: 2026-10-28. Decisão: aprovada pelo responsável pela entrega.

A v0.1.0 é uma entrega funcional focada em Codex. O usuário escolheu explicitamente
essa opção e adiou a validação do Claude. O adaptador Claude permanece disponível como
experimental; não é anunciado como validado nem bloqueia esta versão.

Permanecem obrigatórios: execução real do Codex, gates externos, memória em sessões novas,
isolamento de projetos, captura sem duplicação, migração validada, CI macOS/Linux, checksums,
MIT, repositório privado e revisão humana da revisão final antes da publicação.

Esta decisão substitui apenas o requisito de aceitação real dos dois clientes para a
v0.1.0 nas specs 004, 008, 010 e 012. A aceitação do Claude será uma entrega posterior explícita.
Não há mudança de visibilidade, licença, runtime Docker/Colima ou publicação no crates.io.
