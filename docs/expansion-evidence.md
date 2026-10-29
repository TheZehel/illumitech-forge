# Reconciliação da expansão — 2026-10-29

Consultas read-only à API GitHub (`gh release view` e `gh run view`) confirmaram:

| Evidência | Resultado e alcance |
| --- | --- |
| [v0.1.0](https://github.com/TheZehel/illumitech-forge/releases/tag/v0.1.0) | publicada, não draft, em 2026-10-28T19:47:18Z |
| [v0.2.0](https://github.com/TheZehel/illumitech-forge/releases/tag/v0.2.0) | publicada, não draft, em 2026-10-29T15:36:59Z |
| [CI 40917514708](https://github.com/TheZehel/illumitech-forge/actions/runs/40917514708) | success; revisão d015ff1a283b946e5f8b83fdb8e2c49e72fd9753 |

A CI passou em macos-14 ARM64, ubuntu-24.04 x86_64 e windows-2025 MSVC x86_64:
fmt, Clippy, testes, docs, build, embalagem, licenças e upload de artefatos.
Windows também passou PATH, instalação/preservação e preparação do piloto adiado.
Isso comprova execução automatizada nativa no runner, não piloto autenticado Windows 11,
Ctrl+C interativo, Git/gh reais ou aceite humano de uma release Windows.
O ZIP `illumitech-forge-v0.3.0-windows-candidate-x86_64-pc-windows-msvc.zip` foi baixado
do artefato dessa CI e verificado sem executar/instalar: 5.537.335 bytes, SHA256
`c76e2680fc7f05c33137cb412ccbc6e7b16a25c18bd349821a1c1169fe109e32`,
igual ao sidecar; todos os 396 hashes do SHA256SUMS interno conferem.
A instalação/preservação continua sendo evidência da CI naquela revisão, não deste host.
Mudanças posteriores precisam de novos gates.

`distribution-result.json`, `distribution-v0.2.0.json` e os ledgers de verificação são
snapshots anteriores à publicação; permanecem intactos. Os recibos GitHub posteriores
resolvem seus bloqueios históricos de publicação, não concedem aprovação para nova revisão.
F011-T02 e F012-T02 são reconciliados com esses recibos e os testes integrados já registrados.
F013-T05 já estava concluído tecnicamente; configuração cotidiana não é inferida da release.
F014-T01–T04 refletem inventário, implementação e CI; F014-T05 permanece aberto.

No fechamento da tarefa inicial de especificação, as specs 015–019 ainda não tinham implementação.
Claude continua experimental, novamente adiado pelo usuário; não há spec de homologação
Claude nem login/validação. Memória segue opcional. Os limites históricos das specs 001–014
continuam válidos para suas entregas; as novas specs ampliam apenas o escopo futuro.

## Dependências do contrato selecionado

O manifest externo inicial `illumitech-forge-expansion-20261029` selecionou:
FORGE-EXP-SPECS → DASHBOARD (015) → SERVICE (018) → OBS (019) → DISTRIBUTION (017)
→ MEMORY (016), com prefixo FORGE-EXP em todos os IDs. A numeração das specs identifica
frentes, não a ordem de execução. Dependências técnicas estão em cada spec; checkboxes
não importam conclusão para o ledger operacional.

O primeiro run concluiu specs, dashboard e serviço; observabilidade parou por indisponibilidade
temporária do modelo configurado. O manifest `illumitech-forge-expansion-continuation-20261029`
retomou OBS → DISTRIBUTION → MEMORY com o mesmo modelo e concluiu os três, preservando o histórico
da falha. Nenhum ledger foi editado para inventar conclusão.

## Verificação posterior à implementação

- Gates locais do workspace: fmt, Clippy sem warnings, 78 testes Rust e documentação passaram.
- Migração reutilizável: 16 testes offline; distribuição: cinco testes Python e build dos quatro
  pacotes extraídos. A resolução com patches locais não comprova upload/resolução do registry.
- [Recibo operacional](expansion-runtime-acceptance.json): dashboard no Chrome com estado real,
  serviço launchd com crash/restart/pausa e preservação, Codex real com avaliador externo e
  memória nativa 3.2.9 após cutover. A VM Colima dedicada foi parada; MCP continuou respondendo.
- A revisão MCP real corrigiu `page_id` e o metadado adicional `scope.resolved_by` no verificador;
  os testes mantêm recusa de identidade, conteúdo ou escopo divergente.
- OSV: 222 dependências consultadas, nenhum advisory retornado na data do recibo.
- Auditoria heurística inicial: sete ocorrências revisadas — caminho fictício de exemplo Windows
  e padrões do próprio detector. Achados brutos e revisão permanecem em recibos externos.

Pendentes de gates próprios: CI da revisão final, piloto autenticado Windows, serviços nativos
Linux/Windows, execução integral do tooling genérico de migração em ensaio controlado,
ingestão real LangSmith, autenticação/ownership crates.io, aceite humano da revisão e publicação.
O runtime local migrado não equivale a homologação de toda configuração possível do tooling.
