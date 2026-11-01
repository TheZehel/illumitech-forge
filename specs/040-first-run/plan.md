# Plano

1. Implementar demonstração Rust usando o mesmo motor e contratos existentes; usar somente
   std, Serde, Clap e adaptadores já presentes. Manter comando/avaliador fora da raiz gravável.
2. Acrescentar diagnóstico de seleção para `forge` e `illumitech-forge`, preservando campos
   existentes do JSON schema 1. Limitar a inspeção; versões externas permanecem desconhecidas.
3. Revisar documentação de instalação, teste sem inferência, conflitos e escopo de suporte.
4. Validar fonte congelada por Forge com avaliadores e estado externos: fmt, Clippy, testes,
   docs e um piloto real de comando/gate sem modelo. Reutilizar cache isolado, um job de build
   e testes seriais; não solicitar CI. Logs de falhas são preservados.

Manter checkouts anteriores, identidade humana, dependências/lockfile e configuração nativa.
Os agentes não aprovam evidências textuais. O relatório externo contém achados, correções e
limites observados; a lista de tarefas não substitui o SQLite operacional.
