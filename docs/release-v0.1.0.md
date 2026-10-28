# Illumitech Forge v0.1.0

Primeira entrega funcional, privada e licenciada sob MIT. **Codex validado; Claude experimental.**

O Forge executa tarefas descritas em manifests, registra estado e evidências em SQLite e
exige gates verificáveis para aceitar resultados. Texto do agente não comprova conclusão.

## Incluído

- CLI Rust: preparação, specs, inspeção/dry-run, execução, status, pausa, retomada,
  cancelamento, aprovação humana por revisão e publicação explícita.
- Codex nativo com permissões explícitas e autenticação/modelo preservados.
- ai-memory via MCP: escopo verificado, contexto limitado e bloqueio quando obrigatório.
- Catálogo seletivo, importação revisável do Loop e avaliadores fora da área gravável do agente.
- Binários macOS ARM64 e Linux x86_64, checksums, MIT, licenças de dependências,
  guia bilíngue e exemplos executáveis sem compilar Rust.

## Validação

27 testes automatizados por plataforma, formatação, Clippy e documentação na CI.
Codex real implementou e revisou fixtures; avaliadores externos verificaram comportamento
e preservação. Sessões novas comprovaram recuperação de contexto, captura sem duplicação
e isolamento entre projetos. A migração ai-memory 3.2.9 manteve Docker/Colima e rollback separado.

Consulte [as evidências](https://github.com/TheZehel/illumitech-forge/blob/main/docs/verification.md)
e o [guia de primeira execução](https://github.com/TheZehel/illumitech-forge/blob/main/docs/quickstart.md).
Verifique `SHA256SUMS` antes de extrair o arquivo da sua plataforma.

## Limites conhecidos

- Claude é experimental e não foi homologado nesta versão; não é necessário para usar Codex.
- Linux requer glibc 2.39+ (Ubuntu 24.04). Windows não tem suporte operacional inicial.
- Binário macOS sem notarização. Execução em primeiro plano; daemon e dashboard ficam para depois.
- Codex e ai-memory precisam ser instalados/configurados separadamente; contas/modelos não são fornecidos.
- ai-memory 3.2.9: desative backfill automático no cliente nativo conforme o guia para evitar
  sobreposição com a captura inicial dos hooks.
- Sem publicação no crates.io, alteração de visibilidade ou migração automática de projetos legados.

First functional private release: Codex validated, Claude experimental. Includes both platform
archives, checksums, MIT and dependency licenses, and a bilingual first-run guide. See the guide
for requirements, independent acceptance evidence and current limitations.
