# Plano

Separar editor, transcript/Markdown e apresentação do controlador do terminal existente. Reutilizar Ratatui e a ponte App, com parser Markdown fixado. Guardar histórico/edição somente em RAM da sessão; comandos locais não fazem chamadas ao modelo. Medir espera observada sem inventar ETA, porcentagem ou saldo. Preservar validações de root/estado e decisões nativas. Manifests, fixtures, avaliadores e logs fora do worktree; inspect e dry-run antes da validação real pelo Forge, sem CI remota.

Referências consultadas em 31/10/2026:
- https://learn.chatgpt.com/docs/developer-commands#built-in-slash-commands
- https://code.claude.com/docs/en/interactive-mode
- https://code.claude.com/docs/en/commands
