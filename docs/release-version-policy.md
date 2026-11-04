# Política de versões do Forge

Adotamos [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html): MAJOR.MINOR.PATCH.

| Tipo declarado | Critério | Exemplo a partir de 1.3.0 |
|---|---|---|
| fix | Corrige comportamento incorreto preservando compatibilidade | 1.3.1 |
| feature | Adiciona funcionalidade compatível | 1.4.0 |
| deprecation | Marca funcionalidade pública como depreciada | 1.4.0 |
| breaking | Altera contrato de forma incompatível | 2.0.0 |
| docs / internal | Documentação/organização interna sem impacto público, isoladamente | Sem release de pacote |

Em uma entrega mista prevalece major > minor > patch. Minor zera patch; major zera minor e patch. O critério cobre todo o conjunto desde a versão publicada, incluindo funcionalidades ainda não lançadas. Um fix dentro de uma feature não transforma a entrega inteira em patch. O tamanho do diff não define compatibilidade.

São contratos públicos: comandos e opções CLI, respostas JSON documentadas, manifests, APIs Rust públicas e compatibilidade de estado persistente. Os quatro crates são distribuídos com uma versão coordenada e dependências internas exatas. Uma versão publicada é imutável.

## Cálculo e gate

Na compilação que inclui FORGE-054:

```sh
forge release version --changes release/changes.json --json
forge release version --changes release/changes.json --target 1.4.0 --json
```

Sem --target, calcula a proposta. Com --target, exige exatamente a versão calculada: mismatch retorna status blocked e código 2. Resultado compatível retorna 0. Só docs/internal resulta em release_required=false, proposed_version=null; qualquer --target é bloqueado, pois não há release a fazer.

O registro JSON exige schema_version=1, base_version e changes com id, kind e summary. IDs são únicos, com até 96 caracteres ASCII alfanuméricos/hífen/underscore; até 256 mudanças e 4096 bytes por resumo, arquivo regular de até 1 MiB. Campos ou tipos desconhecidos são rejeitados. A política 1 aceita apenas versões estáveis canônicas MAJOR.MINOR.PATCH, major >= 1, sem zeros iniciais, prerelease ou build metadata; overflow bloqueia.

A classificação e a completude do registro precisam de revisão. O cálculo é automático; não é uma análise semântica do código. A saída contém o hash do registro, a prioridade aplicada e a proposta; não concede aceite nem consulta o registro remoto.

## Preparação de releases

1. Confirmar a última versão publicada e a base do conjunto. Registrar todas as mudanças desde essa base em release/changes.json e revisar o impacto de compatibilidade.
2. Executar release version; usar a proposta para preparar versões do workspace, dependências internas exatas, locks, metadados de UI/site e notas.
3. Incluir release version --target VERSAO como gate obrigatório da release, usando um binário avaliador verificado fora do worktree e inputs fixados. A versão alvo deve ser a mesma dos pacotes e da tag; validar essa coerência no gate de distribuição.
4. Exigir evidências atuais, artefatos/checksums e aceite humano da revisão antes de upload. Actions continuam desativadas salvo autorização explícita; o planejador nunca publica ou modifica arquivos.
5. Arquivar o registro avaliado junto às evidências. Antes da próxima release, atualizar a base e substituir as mudanças pelo novo conjunto; não reutilizar entries já publicadas.

A entrega 1.4.0 reuniu campos adicionais no doctor, os dois fixes e o novo comando de política. O registro em `release/changes.json` descreve o próximo conjunto; execute o gate para calcular a proposta atual.
