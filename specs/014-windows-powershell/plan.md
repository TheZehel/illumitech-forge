# Plano FORGE-014

1. Inventariar usos Unix (sinais, grupos, permissões, PID, shell e paths) nos adaptadores, CLI,
   testes e scripts; definir abstrações portáveis antes de trocar implementações.
2. Implementar supervisor Windows com encerramento da árvore e detecção de processos órfãos;
   validar SQLite/locks e snapshots sem enfraquecer os gates existentes.
3. Adaptar descoberta dos clientes, Git/gh e argumentos; fornecer bootstrap/exemplos `.ps1`.
4. Adicionar CI Windows e empacotamento ZIP/checksums ao pipeline existente.
5. Executar piloto real no Windows/PowerShell, incluindo falhas, reconciliação e preservação.
6. Atualizar matriz de suporte e submeter a revisão exata ao aceite antes da release.

Estado inicial: escopo documentado, implementação e aceitação pendentes.
A ausência de ambiente Windows não pode ser substituída por mocks ou testes em WSL.
