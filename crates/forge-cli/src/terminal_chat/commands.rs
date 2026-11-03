#[derive(Clone, Copy)]
pub(super) struct Command {
    pub name: &'static str,
    pub description: &'static str,
}
pub(super) const COMMANDS: &[Command] = &[
    Command {
        name: "plan",
        description: "Planejar PEDIDO ou carregar @JSON; revisar antes de executar",
    },
    Command {
        name: "run",
        description: "Autorizar o digest do plano e executar o escopo",
    },
    Command {
        name: "tasks",
        description: "Listar tarefas, diagnosticar ID ou /tasks resume ID",
    },
    Command {
        name: "report",
        description: "Evidências e consumo do workflow atual ou ID",
    },
    Command {
        name: "help",
        description: "Ajuda e atalhos de teclado",
    },
    Command {
        name: "status",
        description: "Projeto, modelo, permissões e tokens observados",
    },
    Command {
        name: "sessions",
        description: "Escolher uma conversa deste projeto",
    },
    Command {
        name: "resume",
        description: "Retomar a conversa atual ou /resume N",
    },
    Command {
        name: "new",
        description: "Começar outra conversa",
    },
    Command {
        name: "history",
        description: "Buscar pedidos desta sessão · Ctrl+R",
    },
    Command {
        name: "activities",
        description: "Inspecionar chamadas e diffs individualmente · F2",
    },
    Command {
        name: "reconnect",
        description: "Reconectar sem reenviar mensagens",
    },
    Command {
        name: "quit",
        description: "Sair do Forge · Ctrl+Q",
    },
    Command {
        name: "exit",
        description: "Sair do Forge",
    },
];
pub(super) fn matching(text: &str) -> Vec<Command> {
    let Some(query) = text.strip_prefix('/') else {
        return Vec::new();
    };
    if query.chars().any(char::is_whitespace) {
        return Vec::new();
    }
    COMMANDS
        .iter()
        .copied()
        .filter(|c| c.name.starts_with(query))
        .collect()
}
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Panel {
    Help,
    Status,
    Sessions,
    History,
    Workflow,
    Activities,
}
