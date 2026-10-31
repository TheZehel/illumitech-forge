# FORGE-038 — Experiência de terminal Illumitech

Melhorar leitura e descoberta de recursos do Forge no terminal com identidade ASCII própria, Markdown estilizado, ajuda e comandos acessíveis pelo teclado. Referências de interação: menus de comandos/status do Codex e edição/histórico/ajuda do Claude Code; sem copiar marcas, código ou prometer paridade integral.

## Aceites
- AC1: boas-vindas ASCII adaptativas, status legível, prompt responsivo e telas estreitas sem panic/overflow; conteúdo da resposta permanece nativo.
- AC2: Markdown para títulos, ênfase, listas e código; texto de ferramentas isolado e recolhível. HTML, imagens e sequências de controle nunca são executados ou carregados.
- AC3: menu / filtrável, setas, Tab e Enter; somente comandos realmente implementados. F1 e ? vazio abrem ajuda, /status mostra valores observados e ausências explícitas.
- AC4: histórico apenas da sessão autorizada, busca Ctrl+R, restauração de rascunho, Ctrl+A/E/U/K/W/Y/J e multiline; nenhum histórico adicional persistido nem chamada de reescrita/inferência.
- AC5: aprovações reais continuam explícitas; erro, desconexão e interrupção não viram sucesso ou retry. Operações locais de leitura disponíveis sem reenvio durante turno.
- AC6: gates externos, testes Rust/UI/PTY, instalação Cargo e arquivos de distribuição passam. Publicação, Claude real e piloto Windows ficam fora desta rodada.
