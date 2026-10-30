# FORGE-017 — Abertura pública e crates.io

## Limites e evidência
Especificação autorizada em 2026-10-29; implementação e aceitação desta frente pendentes.
Claude permanece experimental, explicitamente adiado: sem login ou validação nesta entrega.
ai-memory é opcional; somente `required: true` exige disponibilidade e escopo comprovados.
Estado, avaliadores e recibos operacionais ficam fora de repositórios graváveis pelo agente.
Prosa do agente, checkbox e telemetria não substituem gates externos vinculados à revisão.
Esta especificação não autoriza publicação, instalação global, ativação de serviço ou migração de produção.

## Dependências
FORGE-011 e FORGE-013; FORGE-019 precede esta frente no manifest selecionado. Aceite Windows segue FORGE-014.

## Intenção e contrato
Preparar distribuição pública revisável preservando MIT, avisos e autoria humana configurada.
Separar preparação técnica, alteração de visibilidade e upload ao registry em gates independentes.
A autorização das specs não publica nada. Releases privadas v0.1.0/v0.2.0 não provam prontidão pública.

## Aceites
- F017-AC1: inventário dos quatro crates, nomes `illumitech-forge-*` quando necessário, dependências
  internas com versão e path, metadados/repositório/README/MIT/notices e include/exclude explícitos.
  Binário continua `illumitech-forge`; imports, scripts e CI acompanham renomes.
- F017-AC2: `cargo package --list` por pacote auditado; compilar pacotes extraídos em ambiente limpo
  e documentar ordem topológica forge-domain, forge-adapters e forge-catalog (independentes) → forge-cli, conforme os Cargo.toml atuais. Não afirmar upload com package/dry-run local.
- F017-AC3: dependências ainda inéditas no registry são limitação explícita do dry-run;
  verificação local pode usar registry isolado/patch temporário fora da fonte, mas deve registrar
  a diferença para resolução real. Nunca usar `--no-verify` como prova de compilação.
- F017-AC4: auditoria de árvore e histórico Git selecionado, segredos, caminhos pessoais/dados internos,
  licenças e artefatos com saída redigida; CI com permissões mínimas. Achado bloqueia abertura;
  reescrita de histórico exige autorização separada, não faz parte da preparação.
- F017-AC5: publicação requer revisão exata, hashes, nomes/ownership do registry verificados no momento,
  autenticação do operador e aprovação explícita de visibilidade/upload. Falha parcial consulta estado
  remoto e não sobrescreve versões. Registrar recibos reais, sem credenciais.
- F017-AC6: testar inventário incompleto, dependência interna sem versão, inclusão de arquivo proibido
  e ordem inválida; preservar gates nativos e suporte publicado (0.2 sem runtime Windows).

## Seleção explícita de visibilidade
`release publish --expected-visibility PRIVATE|PUBLIC` verifica o alvo; padrão PRIVATE.
Nunca altera visibilidade. Ambos os modos exigem os mesmos gates de revisão humana,
fingerprint, tag e checksums. Dry-run local declara que não consultou visibilidade remota.
