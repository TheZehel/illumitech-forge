# Revisão após 1.1.0 — primeira experiência

## Achados e correções da 1.1.1

| Prioridade | Achado observado | Melhoria |
| --- | --- | --- |
| P0 | `init` cria workspace `illumitech` em qualquer projeto | Memória opt-in e configuração existente preservada |
| P0 | Report pode sair 0 com snapshot passado e evidência atual inválida | Exit code respeita `effective_status`; JSON histórico preservado |
| P0 | Quickstart aponta assets 0.2 e afirma Linux x86_64 na entrega atual | Instalação 1.1.0 e matriz real macOS ARM64 |
| P1 | Exemplo sem agente requer Python e arquivos do checkout | `demo create` nativo, descartável, sem inferência |
| P1 | Doctor só descobre o nome longo | Inspeção dos dois nomes, seleção de PATH e versões desconhecidas explícitas |
| P1 | README usa linguagem de pré-publicação para recursos já entregues | Separação entre 1.1.0 publicada e melhorias locais |

## Escopo de validação

Testes novos cobrem o ciclo real e a ausência de estado no preview; preservação de destinos e
artefatos; rejeição de hash inválido; memória opt-in/idempotência; candidatos de PATH nunca
executados. Fmt, Clippy, testes workspace e documentação são gates locais externos via Forge.
Cache de compilação é isolado; builds e testes usam concorrência limitada. Não executar Actions.

## Próximas decisões de produto

- A vitrine pública depende da decisão de visibilidade/documentação pública; esta rodada
  mantém GitHub privado e não publica mensagens ou campanhas.
- Binários públicos e novos alvos precisam de distribuição e pilotos próprios.
- Publicação 1.1.1 autorizada após a revisão, com congelamento, hashes, notas e gates em FORGE-041.
- Métricas de adoção devem distinguir downloads, primeira execução concluída e retorno;
  nenhuma telemetria ou captura adicional é ativada nesta rodada.

O ledger externo e o relatório final registram os resultados observados; este documento
define o escopo, sem antecipar aprovação ou publicação.
