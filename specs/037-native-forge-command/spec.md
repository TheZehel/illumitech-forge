# FORGE-037 — Entrada nativa forge

`forge` é um executável Rust distribuído pelo mesmo pacote Cargo, sem alias, função de shell, Python ou caminho particular. Sem subcomando, abre o chat no projeto atual. Em subpastas encontra a raiz Git; fora de projetos permite selecionar explicitamente um filho Git imediato. Preservar `illumitech-forge` e seus contratos.

## Aceites
- AC1: ambos os binários compartilham implementação; `cargo install` e arquivos de release incluem forge/forge.exe.
- AC2: `forge`, `forge --root PATH` e `forge chat` abrem a conversa; help/version e subcomandos permanecem disponíveis sem iniciar o cliente.
- AC3: estado automático externo e separado por raiz canônica; `--state-dir` explícito prevalece e mantém validações existentes. Sem estado/permissão/autenticação alterados silenciosamente.
- AC4: seleção de projeto explícita, teclado/cancelamento restauram terminal; não inicializar Git ou cliente antes da escolha. Nome conflitante existente nunca é sobrescrito implicitamente.
- AC5: regressões Rust/UI, instalação Cargo isolada, empacotamento com checksums e piloto PTY passam localmente; sem CI remota, publicação ou troca de conta/modelo.
