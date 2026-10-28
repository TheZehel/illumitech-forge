# Plano FORGE-013

Adicionar prioridade e configuração opt-in de automação ao manifest v1 com defaults compatíveis.
Estado de daemon e recibos de publicação usam SQLite e eventos; locks de worktree continuam obrigatórios.
Separar publicação, fila e importação em módulos. Preservar snapshots v1 e impedir reexecução tácita.

Draft PR usa worktree dedicado previamente preparado. Após commit local, revalidar gates na nova
revisão antes de push; aceites humanos são reavaliados. Recuperação de passos externos usa o commit
e a branch declarados, com inspeção do PR existente. Nunca usar force-push ou sobrescrever branches.

Importar Spec Kit para tarefas explícitas e revisáveis, com ID do documento quando disponível;
ID derivado do conteúdo apenas para legado sem ID, com colisão rejeitada. Ordem respeita dependências,
prioridade e posição no manifest. Checkboxes concluídos são ignorados, não aprovados.

Testar publicação com doubles de transporte e Git real, falhas/reinícios de daemon com processos reais,
importação com deslocamento de linhas e ausência de IDs, e memória ausente/required sem fallback.
Piloto real em repositório descartável privado, somente após validar os gates locais.
