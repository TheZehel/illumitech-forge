# Illumitech Forge CLI — candidata 1.0.0

CLI Rust com Workbench Illumitech local, specs/DAG, execução nativa, gates externos, métricas,
orçamentos, paralelismo, revisão, importação de issues e atualização/recuperação verificável.
Esta versão é candidata local; não foi publicada no crates.io. A versão publicada é 0.3.0.

O binário é `illumitech-forge`. Assets da UI, MIT, inventário e avisos acompanham o pacote.
Use `--help` e `--json contracts` para consultar interfaces/versões e limites. O cliente nativo
é instalado/autenticado separadamente, sem fallback para API paga. ai-memory é opcional, por MCP.

```sh
illumitech-forge --state-dir EXTERNAL_STATE release inspect EXTERNAL_MANIFEST
illumitech-forge --state-dir EXTERNAL_STATE release run EXTERNAL_MANIFEST --dry-run
illumitech-forge --state-dir EXTERNAL_STATE release run EXTERNAL_MANIFEST
illumitech-forge --state-dir EXTERNAL_STATE report EXTERNAL_MANIFEST
illumitech-forge --state-dir EXTERNAL_STATE workbench --root GIT_ROOT --port 0
```

Mantenha manifests, avaliadores fixados por hash e estado fora dos worktrees graváveis.
Texto de agente não prova conclusão. A conversa livre é somente leitura; execução usa um
contrato revisado. Publicação, CI, serviços, push/merge e instalação global são explícitos.
Métricas não representam toda a conta; faixas de duração e cache não garantem economia/prazo.

macOS tem pilotos anteriores; a fonte final exige aceite próprio. Linux ARM64 foi exercitado
em VM/contêiner, com sandbox nativo bloqueado nesse laboratório. Windows/PowerShell e Claude
continuam experimentais; Claude foi adiado. Mocks não substituem conta/plataforma nativa.

Documentação completa e guia em português/inglês no repositório
[Illumitech Forge](https://github.com/TheZehel/illumitech-forge), em `docs/v1-candidate.md`.
Copyright Illumitech. Licença MIT; preserve THIRD_PARTY_NOTICES e THIRD_PARTY_LICENSES.
