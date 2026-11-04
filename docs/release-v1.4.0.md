# Illumitech Forge 1.4.0 — diagnóstico e política de versões

## Novidades

- `forge doctor` inclui o diagnóstico de instalação dos dois comandos nativos.
- Links para o mesmo destino não geram falso conflito; a identidade do comando invocado é preservada mesmo quando o destino tem outro nome.
- Scans acima de 128 diretórios informam cobertura parcial, sem afirmar ausência ou unicidade. Candidatos no PATH nunca são executados pelo diagnóstico de instalação.
- `forge release version --changes release/changes.json` calcula a versão a partir de mudanças revisadas. `--target 1.4.0` exige correspondência exata e retorna código 2 em divergência.
- Política documentada: fix compatível → patch; feature/depreciação compatível → minor; breaking → major. Prevalece o maior impacto do conjunto. Docs/internal isolados não exigem release.

## Atualizar após publicação

```sh
cargo install illumitech-forge-cli --version 1.4.0 --locked
forge --version
forge doctor
```

Se o terminal abrir outra cópia, revise o PATH pelo doctor. Uma sessão já aberta mantém seu binário até ser reaberta.

## Validação e limites

Gates locais e hashes dos artefatos estão em `RELEASE-VERIFICATION.json` da release. Versão no código não é recibo de upload. Os quatro crates mantêm MIT, versões coordenadas e dependências internas exatas; o pacote macOS ARM64 inclui ambos os comandos.

A política calcula a partir de declarações revisadas; não prova compatibilidade semântica nem consulta o registry. Não publica, instala ou altera versões automaticamente. Pré-releases e versões major zero não são suportadas pela política 1.

ai-memory continua opcional. Claude e Windows/PowerShell continuam experimentais; não há novo aceite nativo Linux. O site continua como prévia local, sem deployment. Nenhuma chamada de modelo ou execução de Actions é necessária nesta preparação.
