# Roadmap até 1.0.0

Status: as treze frentes têm implementação local na candidata 1.0.0, sem publicação.
Aceite final por revisão e pilotos permanecem no ledger externo. Base publicada: 0.3.0.
Veja [implementação, evidências históricas e limites](v1-candidate.md).
Os marcos abaixo registram a ordem original de produto, não versões intermediárias publicadas.

## Prioridades definidas pelo usuário

1. **P0 — Economia de tokens:** reduzir consumo por entrega aceita, com qualidade mensurável.
   Instrumentação e controle do contexto precedem automações e paralelismo mais amplos.
   [Escopo e aceites](features/token-economy.md).
2. **P0 — Interface Illumitech para usar Codex pelo Forge:** projetos, conversas, execução,
   aprovações e evidências em uma experiência própria. A implementação começará por um
   piloto local de integração; paridade completa com o aplicativo Codex não é presumida.
   [Escopo e aceites](features/forge-workbench.md).
3. **Estimador de duração do loop:** após separar specs e tarefas, informar faixa de tempo,
   confiança e premissas; atualizar o restante durante a execução.
   [Escopo e aceites](features/duration-estimator.md).

As dez propostas anteriores ficam preservadas abaixo. São treze frentes ao todo; os números
representam ordem de produto e as versões são sugestões, sem compromisso de data.

| Ordem | Frente | Resultado esperado | Marco sugerido |
|---|---|---|---|
| 1 | Economizador de tokens | Medir, controlar contexto e reduzir trabalho repetido por entrega aceita | 0.4, evolução contínua |
| 2 | UI Illumitech conectada ao Codex | Sessões e trabalho pelo Forge com visual e navegação próprios | Piloto 0.5; amadurecer até 1.0 |
| 3 | Estimador de duração | Previsão após specs, tempo restante e comparação com realizado | 0.4–0.5 |
| 4 | Criação guiada de funcionalidades | Preparar specs, manifest e operação conforme o projeto | 0.4–0.5 |
| 5 | Planejamento assistido | Cliente nativo propõe tarefas e aceites revisáveis | 0.5 |
| 6 | Templates versionados | Receitas adaptáveis para bug, feature, revisão e release | 0.5–0.6 |
| 7 | Controle operacional no dashboard | Pausar, cancelar, retomar e aprovar com histórico; integrar à nova UI | 0.6 |
| 8 | Orçamento de execução e CI | Limitar tempo, tentativas, agentes e CI; compartilhar métricas com economizador | 0.6 |
| 9 | Reuso seguro de validações | Evidências reutilizadas apenas quando os insumos relevantes continuam válidos | 0.6 |
| 10 | Paralelismo em worktrees | Concorrência limitada com integração final avaliada | 0.7 |
| 11 | Revisão independente | Implementação e revisão com permissões e evidências próprias | 0.7 |
| 12 | Issue → execução → PR | Relacionar demanda, specs, execução e PR sob política explícita | 0.8 |
| 13 | Atualização e recuperação | Instalação, diagnóstico de PATH, backup e migração recuperável | 0.9 |

## Programa executável

O [plano de execução até 1.0](v1-execution-plan.md) decompõe todas as frentes em specs,
42 passos novos, revalidação do estimador e sete checkpoints. Contratos foram preparados
para inspect/dry-run; suites de comportamento serão materializadas pelo coordenador antes
de cada marco. Implementação integral ainda não iniciada.

## Sequência

Começar pelo inventário de consumo e baseline do economizador. Em paralelo de planejamento,
validar o protocolo do cliente e desenhar a UI; escolher stack e empacotamento após o piloto.
A UI deve exibir desde cedo as métricas disponíveis e distinguir conversa livre de execução
sob manifest. Após estruturar cada pedido executável, mostrar a estimativa de duração do loop;
CLI e UI compartilham a mesma previsão. Conversa concluída não implica tarefa aceita.

Expandir automações e paralelismo depois de tornar consumo e aprovações observáveis.
Claude continua adiado; memória permanece opcional. Os gates pendentes do Windows 0.3.0
continuam pendentes e não são resolvidos por este roadmap.

## Gate de 1.0.0

- CLI, manifest e JSON com política documentada de compatibilidade.
- Instalação e recuperação reproduzíveis, incluindo migração de estado e interrupções.
- Aceitação real em cada plataforma anunciada como estável.
- Economizador comparado com baseline: consumo e qualidade publicados separadamente.
- Estimador calibrado contra durações reais, com faixa, incerteza e espera humana explícitas.
- UI validada com conta real, sessões novas/retomadas, recusas de permissão e falhas de conexão.
- Uso cotidiano em projetos reais, limitações registradas e nenhuma publicação implícita.
- Skills especializadas e ai-memory continuam integrações opcionais conforme o contrato.
