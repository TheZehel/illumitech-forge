# Plano FORGE-052

Inventariar destino e política remota somente por leitura. Preparar metadados e notas
da 1.3.0; preservar a disponibilidade documentada da 1.2.0 até publicação. Congelar
avaliadores externos para auditoria e gates locais. Validar a fonte, consolidar a
candidata local sem hooks remotos e verificar distribuição contra sua revisão limpa.
Construir serialmente, reutilizando cache de compilação sem sobrescrever artefatos
publicados. Extrair crates e exigir Cargo.lock para o build independente. Empacotar
binários, avisos e fontes; reler hashes e cobertura antes do aceite para publicar.
