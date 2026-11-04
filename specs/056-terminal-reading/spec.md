# FORGE-056 — Leitura contínua no terminal

- AC1: respostas maiores que a viewport permanecem acessíveis por mouse/trackpad e PgUp/PgDn; Ctrl+Home/End navega início/fim sem enviar o rascunho.
- AC2: novas linhas não deslocam a leitura quando o usuário sobe; seguir o fim é retomado explicitamente ou ao alcançá-lo.
- AC3: resize preserva a linha lógica; histórico acima de 65535 linhas continua acessível.
- AC4: cabeçalho/rodapé compactos e editor proporcional ao rascunho ampliam a área útil; aprovação e erros permanecem visíveis.
- AC5: saída restaura captura do mouse e modo terminal; rascunho, permissões, cliente/modelo e arquivos preservados. Nenhuma inferência real exigida pelo piloto sintético.
- AC6: testes de estado/renderização e PTY real com protocolo sintético, gates externos atuais pelo Forge. Workbench, publicação e instalação global fora do escopo.
