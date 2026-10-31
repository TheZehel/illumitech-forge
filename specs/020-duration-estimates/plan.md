# Plano FORGE-020

1. Ler engine, contratos, store, lifecycle/parity tests e a proposta de produto.
2. Definir módulo de estimativa puro, contrato serializável e amostragem limitada com chaves
   de comparabilidade documentadas. Instantes UTC para relatório e monotonic para intervalos
   ativos quando possível; injetar relógio nos testes para evitar sleeps ou asserts frágeis.
3. Instrumentar subprocessos e gate phases com durações e outcomes auditáveis; suportar
   leitura de estados anteriores via serde defaults ou metadados aditivos compatíveis.
4. Integrar inspect/dry-run, prévia antes de executar, status/report e transições. Separar
   espera de trabalho ativo. Consultas não escrevem. Não confiar no heartbeat como prova de progresso.
5. Testar contratos e aceites observáveis. Documentar faixas iniciais, confiança, métricas,
   limites de histórico/modelo nativo e interpretação do tempo restante.
6. Rodar fmt, clippy, testes e docs. A avaliação externa fica no diretório operacional.

Manter toda a base 0.3.0 e o roadmap copiado. Evitar refatorações fora do estimador e mudanças
nos pacotes/licenças/dependências. Nenhum teste nativo de serviço ou cliente pago é necessário
para uma funcionalidade determinística de tempo.
