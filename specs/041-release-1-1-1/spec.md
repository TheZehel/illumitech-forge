# FORGE-041 — Distribuição 1.1.1

Publicar as melhorias FORGE-040 aprovadas pelo usuário como 1.1.1: quatro crates MIT,
release GitHub privada e pacote macOS ARM64, preservando identidade humana e a cota de CI.

## Aceites

- Versões e dependências internas exatas 1.1.1; inventários e assets incorporados coerentes.
- Fonte congelada em commit humano com skip-ci; gates externos verificam a revisão final,
  os 225 testes, docs, empacotamento extraído e demonstração nativa sem agente.
- Fonte, binários, quatro crates e verificação acompanhados de checksums.
- Publicação autorizada pela mensagem do usuário após revisão do FORGE-040. Registrar
  aprovação para a revisão final; nenhuma aprovação de versão anterior é reutilizada.
- Não executar Actions, mudar visibilidade, alterar modelos/clientes/hooks ou habilitar serviço.
- Confirmar registry e downloads GitHub após cada upload; não repetir upload incerto às cegas.

Windows e Claude permanecem experimentais. A cobertura Linux não é ampliada. A versão
1.1.0 mantém sua evidência histórica; novos comandos não implicam novo aceite de conta nativa.
