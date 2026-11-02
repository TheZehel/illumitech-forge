# Economia observável

O Forge mede chamadas registradas, não toda a conta. Entrada em cache faz parte da entrada. Campos ausentes continuam desconhecidos. Tokens não são convertidos em dinheiro ou saldo de assinatura.

## Comparar entregas

```sh
forge --json --state-dir ESTADO_EXTERNO report manifest.json > report.json
forge eval compare baseline.json candidate.json
```

A comparação exige relatórios schema_version 1. Mostra resultado efetivo, cobertura, tentativas, duração e tokens por entrega aceita quando completos. Falhas, cancelamentos e evidências obsoletas permanecem visíveis.

Para uma comparação compatível, inclua evaluation_context em ambos os arquivos com case, model, client_version, source_commit, verification_sha256 e configuration_sha256. Esses campos são declarações do operador do experimento; o leitor não autentica uma conta ou modelo por receber strings iguais. Eles devem descrever as condições realmente observadas, nunca suposições. Também é possível envolver o relatório em uma chave report.

Sem contexto comparável ou tokens completos, a diferença permanece indisponível. Mesmo com seis execuções compatíveis, o resultado é exploratório, não uma promessa geral de economia.

## Contexto limitado

Novos workflows usam fontes recuperáveis e orçamento inicial de 16.384 tokens estimados. Manifests legados mantêm o padrão full. O estimador de bytes não representa um limite rígido de inferência; ferramentas, instruções nativas e atividades externas podem acrescentar consumo.

Workflows vinculam recibos de planejamento nativo bem-sucedido ao journal de tokens. Tentativas de planejamento falhas ou não vinculadas, hooks e outras sessões permanecem fora desse total, explicitamente. Duração do planejamento pode permanecer desconhecida.
