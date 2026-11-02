# Registro da preparação 1.2.0 — execução integrada e adoção

Este documento registra a preparação local sobre a base 1.1.1. Consulte [as notas 1.2.0](release-v1.2.0.md) e os recibos da release para o estado de publicação. Esta árvore implementa FORGE-042..048 e não é, por si só, uma
publicação ou um atestado de aceite. Fonte, binários e resultados finais são vinculados por
fingerprints e hashes no ledger operacional externo.

## Capacidades

- Workflow com proposta, escopo confirmado por digest, specs create-only, inspect/dry-run e engine.
- CLI `workflow prepare|start|status|diagnose|resume|list|recipes|findings` e `eval compare`.
- Terminal `/plan PEDIDO` usa planejamento estruturado nativo; `/plan @request.json` usa um
  contrato local explícito. `/run DIGEST` concede escopo; `/tasks`, `/report ID` e
  `/tasks resume|pause|cancel ID` controlam trabalho. `/resume` conserva a conversa.
- Workbench com tarefa guiada, propostas reutilizáveis, receitas, confirmação de escopo,
  seleção de tarefas salvas, diagnóstico, comparação e aceite final declarado.
- Escopo, checkpoints e eventos gravados transacionalmente. Verificadores congelados ficam
  fora do projeto e o início verifica a revisão aprovada sob o lock do engine.
- Importação SARIF 2.1.0 para organizar achados; nenhuma sugestão é executada automaticamente.
- Site estático PT/EN em prévia local e roteiro de adoção sem telemetria.

## Limites explícitos

A autorização inicial não satisfaz gates. Aceite humano declarado é uma etapa final separada.
Permissões nativas continuam independentes. Contratos preparados são imutáveis: para um novo
contrato ou fonte alterada antes do início, escolha um novo ID e revise o novo digest. IDs e
históricos antigos permanecem preservados. Personalizações de specs nunca são sobrescritas.

Interrupção antes do checkpoint seguro de execução exige inspeção do journal e um novo
workflow revisado; comandos de resultado incerto não são repetidos. Pausa de preparação ocorre
na fronteira após o baseline. Cancelamento encerra o worker pertencente a esta interface.
Conversa nativa tem seus próprios limites de interrupção de terminais, descritos no Workbench.

Comparações exigem contadores completos, evidência atual e contexto de experimento comparável,
declarado pelo operador. O leitor não autentica uma conta/modelo a partir desses campos.
Não há promessa de economia percentual, preço, saldo de assinatura ou precisão de prazo.

A receita de dependências exige rescan SARIF e testes. Dados importados permanecem não
verificados; uma varredura sem achados não prova cobertura completa, explorabilidade ou ausência
de vulnerabilidades. Não há instalação automática de OSV/Semgrep ou execução de instruções remotas.

Codex é o cliente do piloto curto. Até seis execuções reais, três tarefas em perfis full/bounded,
mesma configuração e verificações externas, sem retries automáticos. Falhas consomem amostras.
Fixtures determinísticas não são anunciadas como aceite de scanners, contas ou plataformas reais.
Windows e Claude continuam experimentais; não há novo aceite nativo Linux nesta rodada.

## Visibilidade e distribuição

Abertura do GitHub concluída pelo proprietário em 02/11/2026 e confirmada como `PUBLIC`.
A licença permanece MIT. Este marco substitui o requisito inicial de manter o repositório
privado; não publica esta árvore local nem os pacotes 1.2.0. Push/merge, release e upload
continuam separados. Nenhuma alteração de configuração do GitHub ou execução de Actions
é realizada por esta implementação.

## Prévia e adoção

```sh
npm run build --prefix apps/forge-site
npm run preview --prefix apps/forge-site
```

O servidor de prévia escuta apenas 127.0.0.1:4174. As páginas não possuem a ponte nativa,
capability, cookies de Workbench, chaves, código privado ou telemetria. Sem domínio e deployment.
A instalação estável continua em 1.1.1; os guias identificam capacidades da candidata.

O [roteiro de adoção](adoption-pilot.md) prepara cinco pilotos externos. Nenhum convite, mensagem,
postagem, publicação de site, novo crate/release ou atualização global é disparado por esta entrega.

## Identidade visual da prévia

A prévia segue a direção oficial Engenharia em evidência do Figma Illumitech: Instrument Sans local, laranja sólido, superfícies neutras Light/Dark, logos originais e raios semânticos. Referências, licenças e hashes estão em `apps/forge-site/src/assets/provenance.json` e `apps/forge-site/THIRD_PARTY_NOTICES`. A composição e o conteúdo são adaptados ao Forge, sem copiar o site institucional inteiro.

Este ajuste é rastreado por FORGE-049. O ledger anterior e seus pacotes permanecem históricos; a prévia atual tem artefatos e evidências próprios, sem novas inferências nativas.
