# Plano

1. Ajustar versão, documentação e assets; conferir dependências/licenças sem atualizar libs.
2. Congelar fonte em commit humano, validar localmente pelo Forge com um job/testes seriais.
3. Verificar pacotes extraídos e pacote otimizado macOS ARM64; preparar notas e SHA256SUMS.
4. Registrar autorização atual para a revisão congelada e inspecionar publicação dry-run.
5. Publicar crates em ordem domain/adapters/catalog/cli e comparar hashes do registry.
6. Atualizar main por fast-forward exato e tag, sem novo merge commit/CI; publicar a release
   privada pelo Forge e verificar os oito downloads. Preservar checkouts antigos.

A publicação decorre da instrução atual: “perfeito, agora pode subir como 1.1.1”. Não há
mudança de comportamento além das melhorias já revisadas em FORGE-040 e dos metadados.
