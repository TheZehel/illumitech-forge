# Illumitech Forge 1.1.0

Esta entrega acrescenta uma entrada Rust nativa `forge`, conversa formatada na web e uma
interface própria no terminal. O repositório GitHub continua privado; os quatro crates são
MIT e seu código é público quando publicado no crates.io. Não há mudança de visibilidade.

## Uso

Após a publicação: `cargo install illumitech-forge-cli --version 1.1.0 --locked` instala
`forge` e `illumitech-forge`. Execute `forge` dentro de um projeto ou selecione um projeto
na interface. O nome longo continua disponível para scripts existentes. Em caso de conflito
com outro produto chamado forge, preserve-o e instale somente `--bin illumitech-forge`.

## Mudanças

- Web: Markdown seguro, código copiável, skeletons, espera contextual e leitura preservada.
- Terminal: identidade ASCII, Markdown, menu / filtrável, ajuda F1, /status, busca Ctrl+R,
  edição readline, multiline, detalhes de ferramentas, interrupção e resize.
- Raiz Git descoberta em subpastas; escolha explícita fora de um projeto; estado automático
  externo por raiz canônica, com --state-dir explícito prevalecendo.
- Dois executáveis no mesmo pacote Cargo e nos arquivos de distribuição, com checksums.
- Memória continua opcional. Conta, modelo e permissões nativas permanecem preservados;
  comandos locais de UI não fazem inferência adicional nem reescrevem respostas.

## Validação e limites

Os gates de fonte, logs e binários são externos ao worktree. A confirmação desta revisão
pertence ao relatório e aos checksums anexados à release, com autorização humana por revisão.
Os cenários determinísticos de terminal usam fixture de protocolo, sem chamar um modelo.
A validação é local, sem novos runs de Actions. Sob carga elevada da máquina, dois testes
legados de cancelamento expiraram em execução concorrente e passaram isolados em 8 segundos;
a rodada final limita o paralelismo, preservando assertions e prazos dos testes.

O pacote binário desta entrega é macOS ARM64. Windows/PowerShell continuam experimentais;
Claude real permanece adiado. Não há novo aceite de conta nativa em Linux nesta entrega.
A interface não oferece todos os recursos dos clientes nativos. Interromper um turno não
promete encerrar terminais nativos em segundo plano. Conversa e permissão de ferramenta
não aprovam gates Forge. Não há alegação de percentual de economia, saldo ou ETA garantido.
