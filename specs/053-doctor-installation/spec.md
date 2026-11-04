# FORGE-053 — Diagnóstico de instalação

- AC1: forge doctor inclui o diagnóstico de ambos os comandos, mantendo os campos existentes.
- AC2: entradas PATH repetidas e links para o mesmo destino não geram falso conflito. Dois destinos distintos exigem atenção.
- AC3: versões desconhecidas permanecem desconhecidas; nenhuma cópia descoberta é executada. Sem rede, instalação ou alteração de PATH pelo diagnóstico de instalação.
- AC4: distinguir ausência, resolução, atenção e inspeção incompleta, com próxima ação e aviso de sessão já aberta; preservar limites e avisos de cobertura.
- AC5: identificar o nome invocado antes da resolução de links, preservando o aviso quando forge aponta para illumitech-forge e outro forge é selecionado no PATH.
- AC6: após o limite de 128 diretórios, não afirmar ausência nem unicidade; conflitos já comprovados continuam como atenção, com cobertura parcial.
