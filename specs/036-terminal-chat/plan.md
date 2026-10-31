# Plano

Usar Ratatui/Crossterm fixados e o App existente diretamente, sem servidor HTTP adicional. Loop assíncrono mantém controles responsivos enquanto uma ação nativa aguarda resposta. Renderização remove sequências de controle de conteúdo não confiável. Usar IDs únicos e journal de operações existente. Parar e aguardar ações pendentes e ponte ao sair; sempre restaurar raw mode e tela alternativa. Validação local com TestBackend e PTY externo. Não afirmar equivalência integral com todos os recursos do cliente nativo.
