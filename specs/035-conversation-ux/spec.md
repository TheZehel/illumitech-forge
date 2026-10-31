# FORGE-035 — Conversa e estados de espera

A conversa deve ser legível como um chat: Markdown seguro para respostas, texto original preservado, atividades recolhíveis e estados em português. Mostrar skeletons na conexão e retomada, envio imediato e espera por resposta sem inventar progresso. Aprovações nativas e evidências Forge continuam distintas.

## Aceites
- AC1: conexão, retomada e envio têm feedback acessível; falha, interrupção e desconexão encerram a espera.
- AC2: Markdown suporta listas, código e tabelas sem HTML ativo, URLs executáveis ou carregamento automático de imagens remotas.
- AC3: envio único por ação, sem duplicação visual do eco nativo; Ctrl+Enter não envia durante turno ativo. Selecionar conversa carregada não retoma o cliente.
- AC4: atualizações preservam leitura do histórico; botão permite voltar à resposta. Movimento reduzido respeitado e viewport móvel sem overflow.
- AC5: autenticação, modelo, permissões, gates e conteúdo da resposta preservados. Nenhuma inferência adicional para reescrita.
- AC6: build, regressões Rust/UI e avaliações externas passam. Publicação e instalação global fora deste escopo.
