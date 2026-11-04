# Piloto externo — 14 dias

Status: preparado; convites e participação ainda não realizados. O usuário envia os convites a 5–10 pessoas de pequenos times/consultorias que já usam agentes em vários repositórios. A release técnica não depende do piloto.

## Convite para envio manual

Estamos avaliando se o Forge ajuda a concluir correções com menos supervisão. Você pode experimentar uma correção real e, se fizer sentido, repetir em outro repositório nos próximos 14 dias? Precisaremos de cerca de uma sessão inicial e um retorno no dia 7. Você escolhe o código e mantém tudo local. A participação é opcional; nenhuma conta, credencial ou código precisa ser compartilhado.

## Consentimento

Antes de iniciar, confirme: participação voluntária; pode interromper sem justificar; não enviar código, prompts, logs, caminhos ou credenciais. Métricas permanecem locais; revise uma prévia antes de exportar. Registre apenas um identificador pseudônimo, versões, resultados e métricas. Remova o registro local quando desejar. Qualquer publicação agregada exige revisão e autorização específica dos participantes. O Forge não envia dados de rede para o piloto.

## Sessão inicial

1. Registrar sistema, versão do Forge e do cliente, permissão usada e número de repositórios (sem seus nomes).
2. Escolher bug real com resultado esperado observável e fonte preservada em cópia isolada.
3. Usar `/bug DESCRIÇÃO; esperado: ...; reprodução: ...` ou o formulário do Workbench.
4. Revisar escopo, comandos e conteúdo do verificador. Autorizar o digest; observar reprodução, correção e verificação.
5. Revisar diff/evidências. Anotar intervenções do criador, minutos humanos manuais, bloqueios e entrega efetivamente aceita. Teste verde isolado não substitui essa revisão.
6. Ver prévia de `workflow export ID --root PROJETO --ops-root OPERAÇÕES`. Após revisão, gravar com `--output relatório-v1.json --human-minutes MINUTOS`. O envio, se desejado, é manual.

## Retorno D7 e D14

Perguntar no D7: conseguiu concluir sem ajuda? Onde parou? Voltou espontaneamente? Qual intervenção foi necessária? No D14: houve segunda entrega concluída? Registrar respostas e ausências, sem inferir que ausência significa sucesso ou fracasso.

Formulário local: pseudônimo; consentimento; início; versões; resultado 1; número de intervenções; minutos humanos (manual); motivo de parada; retorno D7; resultado 2 até D14; cobertura de tokens; decisão de compartilhar após revisão.

## Comparação

Usar cópias equivalentes com mesmos testes, fonte, cliente, modelo e permissões; alternar ordem Forge/fluxo habitual. Declarar condições antes de executar. Registrar falhas e planejamento sem workflow, tentativas e espera. Tokens desconhecidos não são zero; sem cobertura completa ou condições compatíveis não calcular economia percentual. Comparar minutos e intervenções por entrega aceita, com tamanho da amostra explícito.

Relatório local: convidados (manual), iniciaram, concluíram sem criador, retornaram, desistências/ausências, bloqueios, cobertura de métricas, amostras comparáveis e observações qualitativas. Não publicar dados individuais nem alegar validação estatística.

Metas exploratórias: 5 iniciam; 4 concluem sem criador; 3 retornam e concluem outra entrega em até 14 dias. Se ativação/retorno ficarem abaixo disso, priorizar obstáculos observados antes de provedores, catálogo ou infraestrutura. Marketplace, SaaS e novas integrações ficam adiados.
