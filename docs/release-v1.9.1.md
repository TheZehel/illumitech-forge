# Illumitech Forge 1.9.1

Forge passes Claude manifest task prompts after the CLI option terminator. Claude Code
can now read the entire task when the read-only tool list is present, including prompts
that start with an option-like string. Existing model settings and native permissions apply.

For a write task, configure the Claude Code Edit rule for the intended file.
The [executor contract](contracts.md#manifest-and-operational-contracts) includes a
single-file example. Forge reports native permission denials as blocked.

The compatibility scope is manifest tasks on macOS ARM64 with Claude Code 2.1.331.
Guided workflows, independent review, chat and Workbench keep their existing execution
clients. The skill installer supports the existing Codex and Cursor targets.
