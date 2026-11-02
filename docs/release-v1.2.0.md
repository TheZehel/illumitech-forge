# Illumitech Forge 1.2.0

Do pedido à entrega com escopo revisado, execução persistente e evidências atuais.

## Novidades

- Workflow compartilhado por CLI, terminal e Workbench: proposta, confirmação do digest, specs create-only, execução, diagnóstico e retomada.
- Terminal `/plan PEDIDO`, `/run DIGEST`, `/tasks` e `/report ID`; `/resume` preserva o significado de retomar conversa.
- Receitas de feature, bug com regressão externa antes/depois, remediação SARIF com rescan e checks, preparação local de PR. Nenhuma publicação implícita.
- Contratos, avaliadores e estado externos; aceites humanos continuam separados da autorização inicial. Texto do agente não aprova resultados.
- Comparação exploratória de consumo por entrega; tentativas/falhas preservadas, ausências e incompatibilidades explícitas. Novos workflows usam contexto limitado por padrão, sem alterar contratos legados.
- Site estático PT/EN como prévia local, seguindo Engenharia em evidência do Figma: Instrument Sans local sob OFL-1.1, logos oficiais, Light/Dark e acessibilidade. Sem deployment, telemetria ou ponte nativa.
- GitHub público com licença MIT; ai-memory continua opcional.

## Instalar ou atualizar

```sh
cargo install illumitech-forge-cli --version 1.2.0 --locked
"${CARGO_HOME:-$HOME/.cargo}/bin/forge" --version
```

Confira a ordem do PATH se aparecer outra versão. Ambos os executáveis, `forge` e
`illumitech-forge`, são nativos. Preserve outros produtos chamados forge; instale apenas
`--bin illumitech-forge` se houver conflito de nomes.

## Evidências e limites

241 testes Rust, 26 Python, 22 de navegador, três estáticos do site e 12 checks externos de
primeira execução passaram durante a implementação. Seis invocações reais no cliente nativo
configurado passaram: bug, dependências e preparação local de PR, nos perfis full/bounded.
Nenhuma inferência foi repetida após as correções de prévia/configuração dos avaliadores.
Os checks finais de distribuição e seus hashes estão no anexo RELEASE-VERIFICATION.json.

O piloto pequeno não demonstrou economia de tokens: o perfil limitado consumiu de 2,37% a
2,87% mais tokens observados nos três casos sintéticos. Não há conclusão geral, preço de
assinatura ou promessa de economia. Tokens em cache fazem parte da entrada.

Semgrep CE real foi exercitado com regra sintética rotulada, não um advisory real. Não foi
aceite operacional de OSV nem auditoria SAST/SCA completa do Forge. No Homebrew, selecione
um ponto de entrada externo revisado conforme o guia; seu hash não atesta todas as bibliotecas.

Binário macOS ARM64. Windows/PowerShell e Claude continuam experimentais; não há novo aceite
nativo Linux. As seis amostras reais usaram o build de desenvolvimento da mesma fonte de
execução; o build otimizado tem checks funcionais próprios e não recebe alegação de equivalência
de desempenho. Sem Actions nesta publicação, instalação global, novos hooks ou migração de projetos.

A autorização do proprietário cobre esta fonte revisada e seus pacotes atualizados.
Recibos GitHub/crates.io e checksums identificam a revisão efetivamente publicada.
