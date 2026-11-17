# Illumitech Forge 1.9.0

Human duration messages communicate naturally in Portuguese: “O tempo estimado para
finalização é de: 10 minutos e 30 segundos.” Days, hours, minutes and seconds use correct
singular/plural and omit zero units. For example, 124200 seconds becomes “1 dia, 10 horas
e 30 minutos”. Ranges preserve both bounds with “a”; equal displayed bounds use one value.
Lower bounds round down and upper bounds round up. JSON remains in seconds. Confidence,
approval waits and uncertainty retain their contracts. CLI and Workbench share this convention.

The Construct Companion identity now spans README PT/EN boards, Workbench, static PT/EN
site and terminal: cream #FFFAF0, graphite #252525 and orange #FF7A1A. Web typography uses
local Illumi assets with their supplied conditions. Official SVG logos are unchanged;
the terminal uses the configured emulator font with a matching palette and plain borders.

All seven maintained READMEs and their templates are synchronized for 1.9.0, including
current install commands. Boards are illustrative. macOS ARM64 binaries and source crates
are verified from extracted archives. Runtime validation covers 317 Rust tests and 36
Playwright tests, plus formatting, Clippy, Rust documentation and external CLI gates.

This release does not add new native-account or editor acceptance, Linux/Windows acceptance,
a website deployment or a global installation. Existing experimental platform limitations
remain. Web fonts retain their supplied IllumiTech brand-use conditions alongside MIT code.
