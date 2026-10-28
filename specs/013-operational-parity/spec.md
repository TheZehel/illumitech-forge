# FORGE-013 — Adoção com paridade operacional

## Intenção
Fechar as lacunas relevantes do Loop antes de configurar o Forge como ferramenta cotidiana.
A versão 0.1.0 e os manifests, estado, automações e checkouts do Loop permanecem preservados.

## Aceites
- F013-AC1: memória opcional também na publicação; tags e arquivos de qualquer projeto, sem nomes fixos do Forge. Caminhos relativos funcionam. Publicar requer revisão humana atual, tag remota correta e checksums.
- F013-AC2: daemon opt-in com fila determinística, execução serial, pausa/retomada persistentes, heartbeat, lock entre instâncias e inspeção/dry-run sem mutação. Bloqueios e alterações materiais exigem reconciliação explícita.
- F013-AC3: draft PR opt-in após gates, em worktree isolado e branch declarada. Commit usa a identidade humana configurada, nunca Codex como autor/coautor. Falha/reinício não duplica commits ou PRs; nenhum merge automático.
- F013-AC4: importação expande tarefas pendentes de Spec Kit, preserva IDs permanentes/dependências/prioridades e exige revisão de aceites/gates. Estados e checkboxes antigos não viram evidências novas.
- F013-AC5: testes de regressão, CI nas duas plataformas e piloto real sem memória. Configuração cotidiana somente depois dos aceites; publicação/revisão permanecem ações explícitas.

## Limites
Sem dashboard, Windows, troca de runtime ai-memory, migração em massa ou validação Claude.
Daemon supervisiona em primeiro plano e pode ser operado por launchd/systemd; não instala serviços.
Publicação/draft PR de múltiplos repositórios exigem contratos separados por alvo.
