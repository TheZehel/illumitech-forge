# FORGE-039 — Entrega 1.1.0

Publicar a evolução de UX web, terminal e entrada Rust nativa forge como versão 1.1.0, mantendo GitHub privado e licença MIT. Crates públicos e release GitHub são ações explícitas, com aprovação humana vinculada à revisão, sem autoria de agentes ou novos runs de Actions.

## Aceites
- Os quatro crates e UI usam 1.1.0 e dependências internas exatas.
- Gates locais, testes de terminal e pacotes extraídos verificam a fonte congelada; limitar concorrência para a máquina local sob carga, sem relaxar assertions.
- Binários nativos forge e illumitech-forge, checksum de cada arquivo, inventário e licenças completos no pacote macOS ARM64; demais plataformas com cobertura explicitada, sem inventar aceite.
- Notas, revisão, artefatos e hashes revisáveis antes de publicar. Abertura pública do repo não faz parte desta ação.
