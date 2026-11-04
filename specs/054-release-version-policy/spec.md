# FORGE-054 — Política automatizada de versões

- AC1: registro JSON versionado com base estável, IDs únicos, tipo e resumo de cada mudança; falhar em ambiguidade, campos desconhecidos, tipos inválidos, entradas vazias ou limites excedidos.
- AC2: fix → patch, feature/deprecation → minor, breaking → major; docs/internal isolados não geram release; prevalece o maior impacto e os componentes inferiores são zerados.
- AC3: release version calcula a proposta e --target valida a versão exata, bloqueando subincremento, incremento excessivo e reuso. JSON inclui base, regra, justificativa, proposta e hash do registro.
- AC4: apenas leitura, sem editar versões, publicar, habilitar Actions ou conceder aceite; o registro depende de revisão de compatibilidade e cobertura das mudanças.
- AC5: contratos públicos incluem CLI, JSON, manifests, API Rust e estado persistente. Os quatro crates têm versionamento coordenado. SemVer estável major >= 1; prereleases/build metadata exigem política futura explícita.
- AC6: critérios próprios do Forge documentados em AGENTS; próximo conjunto descreve o doctor aditivo, os dois fixes e o planejador, resultando em 1.4.0 a partir de 1.3.0. Fixes isolados exigiriam 1.3.1.
