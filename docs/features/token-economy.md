# P0 — Economizador de tokens

Registro histórico da descoberta após 0.3.0. Para capacidades atuais, consulte [medição e contexto](../token-economy.md) e [comparação da candidata 1.2](../guides/economy.pt.md).
Área: contexto, execução, uso e avaliação. Repositório: Illumitech Forge.

## Objetivo

Ajudar usuários a gastar menos tokens para obter a mesma entrega aceita. Menos tokens em
uma resposta isolada não comprovam economia se houver mais tentativas, falhas ou retrabalho.
Não prometer percentual de redução antes de medir.

## Contexto atual

O adaptador executa `codex exec --json` e valida eventos de conclusão, mas o resumo em
`crates/forge-adapters/src/clients.rs` não consolida consumo. Há limites de tentativas e
processos, porém isso não limita diretamente tokens dentro de um turno do cliente.
O dashboard é somente leitura. Não há garantia atual de economia de tokens.

## Comportamento proposto

1. **Medir antes de otimizar.** Registrar uso por sessão, tarefa, tentativa e release quando
   informado pelo cliente. Separar entrada, saída e entrada em cache quando disponíveis;
   contadores de reasoning podem ser subconjuntos e não devem ser somados duas vezes.
   Identificar campos ausentes como indisponíveis. Deduplicar atualizações cumulativas.
2. **Preparar contexto seletivo.** Enviar objetivo, critérios, instruções aplicáveis e trechos
   necessários; deixar arquivos completos recuperáveis sob demanda. Remover duplicatas de
   instruções, histórico e memória sem descartar regras, bloqueios, fontes ou aceites.
   Isolar todo contexto por projeto; ai-memory opcional não pode se tornar pré-requisito.
3. **Reduzir chamadas evitáveis.** Usar descoberta local determinística para paths, Git, testes
   e inventários; resumir saídas volumosas para o próximo turno com referências aos logs
   completos. Não alterar stdout estruturado do cliente de forma a invalidar seu protocolo.
4. **Handoff incremental.** Retomar sessão compatível ou construir contexto limitado a mudanças,
   decisões e trabalho pendente. Comparar custo de retomar com custo de iniciar; nem sempre
   uma sessão longa é mais barata. Testar compaction e resumos contra perda de informação.
5. **Limites compreensíveis.** Orçamento por tarefa/release e alertas na CLI/UI. Sem estimativa
   confiável de saldo, mostrar desconhecido. Controle antes de novos turnos é diferente de um
   teto rígido durante a inferência: só oferecer este último se o cliente o suportar e validar.
6. **Perfis opcionais.** Explorar econômico, equilibrado e profundo como políticas explícitas.
   Não trocar modelo, conta, provedor, reasoning ou permissões silenciosamente. Não gerar
   chamadas de LLM só para tentar economizar sem contabilizar seu próprio consumo.

Tokens processados, tokens em cache, dinheiro e limites de uma assinatura são medidas distintas.
Não converter tokens em saldo de assinatura ou valor monetário sem base verificável e datada.
Cache do provedor não é controle local do Forge; registrar somente o benefício observado.
Reuso de evidências depende de hashes, revisão, avaliadores e ambiente relevantes: um aceite
humano anterior não se transfere automaticamente para outra revisão.

## Aceites propostos

- Relatório diferencia consumo observado, estimado e indisponível, incluindo metadados,
  chamadas de planejamento/compaction, tentativas falhas e canceladas quando reportadas.
- Contexto tem limite configurável, fontes recuperáveis e preserva instruções e isolamento.
- Experimentos pareados usam tarefas idênticas, modelo/configuração fixos, mesmos gates
  externos e múltiplas execuções; ordem e condições de cache são registradas.
- Comparar tokens por entrega aceita, taxa de sucesso, retrabalho e latência. Não esconder
  falhas da amostra. Definir margem de qualidade e meta de economia após obter o baseline.
- Redução de consumo não transforma execução incompleta ou sem evidência em sucesso.
- Nenhum fallback pago ou alteração silenciosa de modelo; gasto da própria otimização incluído.

## Dependências e descoberta

Inventariar os eventos da versão instalada de cada cliente. A documentação de App Server
inclui `thread/tokenUsage/updated`, mas sua presença e semântica precisam ser testadas no
cliente selecionado. Descobrir os limites de controle antes de anunciar orçamento rígido.
Primeiro piloto: comparar a execução atual com contexto seletivo e recuperação sob demanda.
Módulos prováveis: adapters/clients e observability, contratos de domínio, engine, report e UI.

Referência verificada em 2026-10-30: [Codex App Server](https://developers.openai.com/codex/app-server/).
Os mecanismos acima são propostas Forge, não garantias da documentação do cliente.
