# Plano FORGE-018

Estado: implementação local em `crates/forge-cli/src/service.rs`; aceite nativo pendente.
Procedimentos e limites: `docs/user-services.md`. A suíte padrão não registra serviços.

1. Reutilizar daemon/locks; separar renderização pura, inventário e aplicação nos adaptadores de plataforma.
2. Modelar plano com hash, identidade e ações reversíveis. Revalidar arquivos e destino no apply/remove.
3. Implementar comandos nativos por argv, sem interpolação de shell; definir execução apenas na sessão
   do usuário, sem armazenar senha, habilitar linger ou mudar políticas da máquina.
4. Testar falhas parciais, ownership e escaping com fixtures e processos substitutos determinísticos.
5. Adicionar instruções e testes nativos opt-in na CI quando houver sessão compatível; nunca instalar
   no host de desenvolvimento ou ativar serviço automaticamente durante a suíte padrão.
Não migrar filas existentes nem habilitar publication automation.

## Validação e entrega
Executar `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` e `cargo doc --workspace --no-deps`. Testes novos devem exercitar
comportamentos positivos e recusas; não desabilitar testes existentes para obter sucesso.
Registrar revisão, comando, resultado e hashes de artefatos em recibos externos sanitizados.
Gates nativos, autenticados ou externos não executados permanecem explicitamente pendentes.
