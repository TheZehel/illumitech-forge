# Tarefas FORGE-019

Todas pendentes; conclusão exige os aceites do spec e evidência externa da revisão.

- [ ] F019-T01: Sink e configuração opt-in; evidência: F019-AC2/AC3; default sem rede e limites.
- [ ] F019-T02: Spans e correlação; evidência: F019-AC1; sequência real, tentativas e resultados preservados. <!-- forge: {"dependencies":["F019-T01"]} -->
- [ ] F019-T03: Transporte e testes fake; evidência: F019-AC3/AC4/AC5; timeout, cancelamento, erro, redaction e ausência de rede. <!-- forge: {"dependencies":["F019-T02"]} -->
- [ ] F019-T04: Aceite LangSmith separado; evidência: F019-AC5; ingestão real autorizada por revisão, permanece pendente. <!-- forge: {"dependencies":["F019-T03"]} -->
