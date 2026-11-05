# Plano F065

Implementar em modules/qa.rs e integrar ao roteamento/terminal existentes. Reutilizar
process::run, redaction, locks, fingerprint, safe_path/external e parser SARIF atuais.
Evitar dependências novas e um segundo scheduler genérico. Persistência QA usa artefatos
externos imutáveis e o Store/journal existente quando apropriado; não editar tabelas ad hoc.

Candidato inclui root canônico, request revisado, fonte, hashes e digest. O comando start
adquire o lock e revalida antes do primeiro processo; registra intenção antes de executar.
Cada check precisa de resultado observado e evidência, com checagem de fonte ao terminar.
Status somente leitura compara estado atual. Contexto de relato não concede nova execução.

Integração terminal compartilha o módulo, não implementa outro executor. Identificar o
plano QA separadamente do plano feature; limpar digest ao trocar escopo/modo. Continuar
mostrando Refatoração/CI/CD como indisponíveis. Não modificar a UI web neste incremento.

Implementação conduzida pelo cliente Codex configurado através de um manifest Forge.
Manifest, estado e avaliador ficam fora deste worktree. Inspeção e dry-run precedem run.
Não commitar, publicar, instalar o produto, alterar hooks, model/account ou mover outras filas.
A instalação local, se necessária, é responsabilidade do operador após gates e revisão.
