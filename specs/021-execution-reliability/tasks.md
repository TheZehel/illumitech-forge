# Tarefas FORGE-021

Status: backlog; marcas de conclusão dependem do ledger externo da revisão.

- [ ] F021-T01: Fixar fixtures externas de protocolo, redação e falso aceite; verificar o estimador existente no novo worktree. Aceites: FORGE-021. <!-- forge: {"dependencies": [], "priority": 210} -->
- [ ] F021-T02: Corrigir adaptadores e continuidade, com testes negativos e preservação de configuração. Aceites: FORGE-021. <!-- forge: {"dependencies": ["F021-T01"], "priority": 211} -->
- [ ] F021-T03: Validar CLI e cliente real em repositório descartável; bootstrap deve usar o código corrigido antes dos jobs nativos seguintes. Aceites: FORGE-021. <!-- forge: {"dependencies": ["F021-T02"], "priority": 212} -->

## Regressões mínimas previstas

- `structured_logs_keep_valid_json_and_redact_secrets`
- `plain_text_never_completes_a_task`
- `retry_policy_refuses_permission_and_contract_failures`

Nomes são contratos de cobertura a materializar em suites externas antes do marco;
não são testes existentes nem evidências de implementação.
