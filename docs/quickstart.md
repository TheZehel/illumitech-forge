# Primeira execução / First run

O binário extraído roda sem Rust. Requer Git e Python 3 para os exemplos.
macOS ARM64 e Linux x86_64 com glibc 2.39+ são os alvos desta entrega.
Use um diretório novo para cada exemplo; os scripts preservam diretórios existentes.

The extracted binary needs no Rust toolchain. The examples require Git and Python 3.
Use a new example directory each time. Linux requires glibc 2.39 or newer.

## Verificar o download / Verify the download

Baixe o arquivo da sua plataforma e `SHA256SUMS` da release privada no GitHub.
Na pasta dos downloads, confira a linha correspondente antes de extrair:

Download your platform archive and `SHA256SUMS` from the private GitHub release.
Verify the matching line before extracting:

```sh
# macOS ARM64
grep 'aarch64-apple-darwin.tar.gz$' SHA256SUMS | shasum -a 256 -c -
tar -xzf illumitech-forge-v0.1.0-aarch64-apple-darwin.tar.gz

# Linux x86_64: use these two commands instead
grep 'x86_64-unknown-linux-gnu.tar.gz$' SHA256SUMS | sha256sum -c -
tar -xzf illumitech-forge-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
```

Os comandos seguintes partem da pasta onde você extraiu o arquivo.
Run the following commands from the extraction directory.

## Sem agente / Without an agent

```sh
./illumitech-forge --version
./illumitech-forge doctor
python3 scripts/demo.py /tmp/forge-first-run
./illumitech-forge --state-dir /tmp/forge-first-run/state release inspect /tmp/forge-first-run/release.yaml
./illumitech-forge --state-dir /tmp/forge-first-run/state release run /tmp/forge-first-run/release.yaml --dry-run
./illumitech-forge --state-dir /tmp/forge-first-run/state release run /tmp/forge-first-run/release.yaml
./illumitech-forge --state-dir /tmp/forge-first-run/state report /tmp/forge-first-run/release.yaml
```

Resultado esperado: `passed`, com `hello.txt` validado por um avaliador externo.
Expected result: `passed`, with `hello.txt` checked by an external evaluator.

## Com Codex real / With native Codex

Instale e autentique o Codex CLI previamente. O Forge usa sua configuração, modelo e
permissões nativas. Esta execução usa sua conta; não há troca automática para outra API.
O script prepara fixtures de ambos os clientes, mas os comandos abaixo executam somente Codex.

Install and authenticate Codex CLI first. Forge preserves its configured model, account and
permissions. The fixture script prepares both clients; the commands below run only Codex.

```sh
python3 scripts/agent-fixtures.py /tmp/forge-native-demo
./illumitech-forge --state-dir /tmp/forge-native-demo/state release inspect /tmp/forge-native-demo/forge-eval-codex-baseline.yaml
./illumitech-forge --state-dir /tmp/forge-native-demo/state release run /tmp/forge-native-demo/forge-eval-codex-baseline.yaml --dry-run
./illumitech-forge --state-dir /tmp/forge-native-demo/state release run /tmp/forge-native-demo/forge-eval-codex-baseline.yaml
./illumitech-forge --state-dir /tmp/forge-native-demo/state report /tmp/forge-native-demo/forge-eval-codex-baseline.yaml
```

O agente implementa uma soma, preserva arquivos existentes e faz revisão somente leitura.
O avaliador verifica inteiros e números fracionários. Um handoff falso não satisfaz o aceite.
A memória é opcional neste exemplo; configure MCP e `memory.required: true` para exigi-la.

The agent implements addition, preserves existing files and performs a read-only review.
An independent evaluator checks behavior. Memory is optional in this example.

## Memória e limites / Memory and limits

- No ai-memory 3.2.9, use `backfill_on_start = false` no `config.toml` do cliente nativo
  para evitar que a importação automática de histórico concorra com a captura da primeira
  sessão. O importador manual continua disponível para uso deliberado. Os hooks ficam ativos.
- Revise os hooks do ai-memory em `/hooks` no Codex. Não use bypass de confiança.
- Configure workspace e projeto explicitamente; `memory doctor` deve confirmar o mesmo escopo
  usado pelos hooks. Um projeto novo precisa de uma sessão nativa antes de exigir memória.
- Claude permanece experimental e sem aceitação real nesta versão. Não é requisito para usar Codex.
- Publicação exige uma ação separada e um aceite humano vinculado à revisão. Não ocorre ao terminar tarefas.
- O binário macOS não é notarizado; esta entrega não inclui instalação automática nem serviço em background.

For ai-memory 3.2.9, disable automatic startup backfill in the native client's configuration,
review hook trust in Codex and verify the exact project scope. Live hooks remain enabled.
Claude is experimental. Publishing is explicit and revision-approved. The macOS archive is not notarized.
