# Dashboard local

```sh
illumitech-forge --state-dir /caminho/externo/forge-state dashboard
# Porta fixa opcional:
illumitech-forge --state-dir /caminho/externo/forge-state dashboard --port 8787
```

Use um diretório fora de qualquer repositório Git, inclusive ao usar symlinks. O comando
recusa estado dentro de repositórios. A porta padrão `0` pede uma porta livre ao sistema;
o endereço `http://127.0.0.1:<porta>/` aparece no terminal. Abra esse endereço exato no
navegador. `localhost`, proxies e hosts alternativos são recusados. Ctrl+C encerra o servidor.
Nenhum navegador, agente ou daemon é iniciado automaticamente.

A interface em português apresenta releases, tarefas, tentativas, evidências dos gates e
eventos. Selecione uma tarefa para consultar seus gates. Resultados ausentes aparecem como
não executados, nunca como aprovação. Heartbeat com mais de 30 segundos, futuro ou ausente
não confirma atividade; heartbeat recente também não comprova progresso. Atualize a página
para obter outro snapshot. A leitura de cada página usa uma transação SQLite consistente.

O banco `state.sqlite` é aberto somente leitura, sem criação ou migração. Um banco ausente
produz estado vazio; incompatibilidade, bloqueio, corrupção ou excesso de dados produzem
erro sanitizado. SQLite pode gerenciar seus arquivos auxiliares WAL/SHM; o dashboard não
modifica registros operacionais. Não há comandos, aprovação, cancelamento ou acesso a
arquivos pelo HTTP. `/health` confirma apenas que o servidor responde, não a saúde dos gates.

As listas usam até 100 releases/tarefas/evidências e 200 eventos por página. Releases e tarefas
usam cursor por chave; eventos por sequência; evidências usam posição no array registrado.
Se uma reavaliação substituir evidências entre páginas, volte à primeira página da tarefa.
O JSON de uma release é limitado a 1 MiB antes de ser carregado; respostas HTML também
são limitadas a 1 MiB. Consultas têm prazo de 500 ms (verificado a cada 1.000 instruções SQLite),
espera de lock de 200 ms e conexões HTTP têm prazo total de 3 segundos. Uma conexão é
atendida por vez; cabeçalhos são limitados a 8 KiB. Identificadores exibidos são limitados a
128 caracteres e escapados; URLs usam IDs codificados em hexadecimal, nunca paths de arquivos.

Somente campos de identificação/status, tentativas, resultados, sequência e data dos eventos
são exibidos. Prompts, memória, manifests, ambiente, paths de revisão, detalhes de evidência e
corpos dos eventos ficam fora do HTML. Não coloque segredos em identificadores operacionais.
Tipos de evento e status desconhecidos recebem rótulos fixos. Não há JavaScript, CDN, CORS
ou recursos remotos; CSP impede scripts, frames e formulários. Host e Origin são verificados
contra o endereço efetivo, inclusive contra DNS rebinding. Este servidor é para inspeção local;
não exponha sua porta por túnel ou proxy.

Os testes HTTP usam bancos descartáveis externos e verificam escape, exclusão de campos
privados, Host/Origin, métodos, rotas, limites, paginação, leitura durante escrita WAL,
estados vazios/corrompidos e preservação do hash do banco sem escrita concorrente.
Validação de plataforma nativa não executada, autenticação de provedores, piloto Windows,
publicação e aceite externo da revisão permanecem gates separados. Claude segue experimental,
sem login ou validação nesta tarefa. A memória continua opcional.

## Verificação local — 2026-10-29

- `cargo fmt --check`: passou.
- `cargo clippy --workspace --all-targets -- -D warnings`: passou.
- `cargo test --workspace`: 50 testes passaram, incluindo 6 do dashboard; nenhum ignorado.
- `cargo doc --workspace --no-deps`: passou.
- Navegador local com fixture sintética externa: páginas de release/tarefas/gates/eventos,
  larguras 360 e 1280 px sem overflow horizontal da página, foco visível no link de salto,
  nenhum script. Colunas ajustadas após inspeção mobile.
- Comando real em porta efêmera e Ctrl+C: servidor encerrado com `status: stopped`.

O sandbox inicialmente impediu o bind loopback; os testes HTTP foram executados com
permissão nativa local, sem remover ou ignorar testes. Esses resultados locais não substituem
os recibos do avaliador externo vinculados à revisão nem os gates pendentes acima.
