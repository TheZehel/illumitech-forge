# Illumitech Forge 1.1.1

Correções de primeira experiência e automação, após revisão local de FORGE-040.

## Mudanças

- `demo create DIRECTORY` prepara um exemplo nativo novo, sem Python, agente, conta ou
  memória; comandos explícitos para inspect, dry-run, run e report. Gate externo por hash,
  evidência real e preservação de arquivos. Git herdado com roteamento é recusado.
- `init` não cria memória por padrão. `--memory-workspace ID` é opt-in e preserva roteamento
  existente. O namespace `illumitech` deixa de ser imposto a novos projetos.
- `installation doctor` inspeciona os dois nomes e seleção de PATH, sem executar candidatos
  desconhecidos ou presumir suas versões; declara aliases/ACLs/cache fora do diagnóstico.
- `status`/`report` usam `effective_status` para o exit code: evidência obsoleta retorna 2,
  mesmo se o snapshot histórico disser `passed`. JSON e comportamento somente leitura mantidos.
- README PT/EN, início rápido e suporte alinhados à distribuição real.

## Instalar/atualizar

```sh
cargo install illumitech-forge-cli --version 1.1.1 --locked
"${CARGO_HOME:-$HOME/.cargo}/bin/forge" --version
```

Confira PATH se outra cópia ainda aparecer. Ambos os executáveis são nativos; preserve
outros produtos chamados forge e use --bin illumitech-forge se precisar somente do nome longo.

## Evidências e limites

A revisão local anterior passou em 225 testes Rust, 26 Python e 12 verificações funcionais,
com fonte e avaliadores externos. Os resultados da fonte final 1.1.1 estão no anexo
RELEASE-VERIFICATION.json; números de versão não são atestados de aceite.
Quatro falsos positivos históricos do Gitleaks foram classificados (três hashes de fonte e
prosa), com exceções exatas protegidas por hash integral dos arquivos. Não é auditoria de
histórico Git ou segurança completa. As dependências externas permanecem inalteradas.

Pacote binário macOS ARM64; Windows/PowerShell e Claude permanecem experimentais. Não há
novo aceite Linux com conta nativa. O protocolo/conta/modelo nativos permanecem preservados;
esta rodada exercita os novos fluxos sem inferência. Não há alegação de economia percentual.

GitHub permanece privado; o código MIT dos quatro crates é público. A publicação não
habilita serviços, instala hooks, modifica PATH ou atualiza instalações globais. Sem novos
Actions autorizados. As notas, fonte e assets são vinculados por hashes ao commit final.
