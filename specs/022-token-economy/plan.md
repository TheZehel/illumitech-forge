# Plano FORGE-022

Módulos prováveis: adapters/clients; domain; engine; report; contexto; avaliações.

1. Implementar coleta local e baseline reproduzível; registrar limitações de cada cliente.
2. Implementar contexto limitado e perfis explícitos, sem novos serviços pagos ou hooks.
3. Comparar baseline e otimização em fixtures equivalentes com gates externos; registrar resultados e qualidade.

## Validação

Fixtures e avaliadores ficam fora de worktrees graváveis. Cada suite externa é preparada e
fixada pelo coordenador antes da execução do marco; fixture ausente bloqueia, nunca aprova.
Checks direcionados durante implementação; fmt, Clippy sem warnings, workspace tests e docs
no checkpoint integrado. UI exige build/typecheck e browser real; recursos remotos exigem piloto
declarado. Mocks são regressões, não substitutos da aceitação real.

Mudança material de contrato, avaliador ou worktree invalida evidências; atualizar candidatos,
rodar inspect/dry-run e reconciliar com motivo antes de retomar. Não pedir ao usuário decisões
de biblioteca/organização rotineiras que o contrato já permite.

## Contratos do primeiro incremento

- `report` e `status` acrescentam `token_usage`: input_tokens, cached_input_tokens,
  output_tokens e total_tokens, opcionais quando indisponíveis. Cache faz parte da entrada;
  total não soma cache novamente. Deduplicar completion repetido da mesma sessão/turno.
- `context plan MANIFEST --task ID --budget-tokens N` produz context_plan com sources (path
  relativo e proveniência), orçamento/estimativa e limitações. Instruções obrigatórias não
  são omitidas para caber; se excederem, mostrar bloqueio/oversize explícito.
- Preserve o protocolo válido corrigido. Configuração de contexto opcional no manifest deve
  ser aditiva/default conservador; uso do cliente e aprovações existentes permanecem intactos.
- Gates com eventos artificiais provam regressões. Comparação nativa real é evidência distinta;
  não apresentar estimativas de caracteres ou fixtures como redução real de uso da assinatura.
