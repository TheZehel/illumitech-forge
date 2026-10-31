# FORGE-036 — Conversa interativa no terminal

Disponibilizar `illumitech-forge chat` como interface própria Illumitech no terminal, com conversa em streaming, composer, sessões, indicadores de espera, uso e decisões nativas explícitas. Complementa a UI web, compartilhando a ponte e o histórico nativo sem simular o renderer do cliente.

## Aceites
- AC1: terminal interativo com identidade Forge, entrada Unicode/multilinha, loading, rolagem e atalhos documentados; falhas e saída restauram terminal.
- AC2: mesma App/ponte do workbench; autenticação/modelo herdados, read-only/on-request; nenhum bypass, reenvio ou API adicional.
- AC3: sessões recuperáveis explicitamente, decisões limitadas às opções nativas, interrupção sem alegar encerrar terminais em segundo plano.
- AC4: testes determinísticos de renderização/teclas e piloto PTY com fixture de protocolo; diferenciar fixture de aceite com conta real. Sem publicação/instalação automática.
