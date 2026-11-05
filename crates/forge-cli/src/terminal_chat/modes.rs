//! Local mode selection changes routing, never native permissions or execution consent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Mode {
    Feature,
    Qa,
    #[default]
    Conversation,
}
pub(super) const OPTIONS: &[(&str, &str, Option<Mode>)] = &[
    (
        "Feature",
        "Planejar, revisar e executar uma funcionalidade",
        Some(Mode::Feature),
    ),
    (
        "QA",
        "Preparar REQUEST, revisar escopo e executar checks explícitos",
        Some(Mode::Qa),
    ),
    ("Refatoração · em breve", "Ainda indisponível", None),
    ("CI/CD · em breve", "Ainda indisponível", None),
    (
        "Conversa · somente leitura",
        "Explorar o projeto sem executar um workflow",
        Some(Mode::Conversation),
    ),
];
impl Mode {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Feature => "Feature",
            Self::Qa => "QA",
            Self::Conversation => "Conversa · leitura",
        }
    }
    pub(super) fn hint(self) -> &'static str {
        match self {
            Self::Feature => {
                "Descreva a funcionalidade. Enter cria um plano para revisão; não executa a implementação."
            }
            Self::Qa => {
                "Informe o caminho do REQUEST JSON. Enter prepara; /qa-run DIGEST confirma. /qa NÍVEL MANIFEST mantém planejamento legado."
            }
            Self::Conversation => "Converse sobre o projeto. /mode troca o modo de trabalho.",
        }
    }
    pub(super) fn editor_label(self) -> &'static str {
        match self {
            Self::Feature => "Feature · Enter planeja · Ctrl+J nova linha",
            Self::Qa => "QA · REQUEST JSON · Enter prepara",
            Self::Conversation => "Mensagem · Enter envia · Ctrl+J nova linha",
        }
    }
    pub(super) fn route(self, text: &str) -> String {
        let explicit_command = text.split_whitespace().next().is_some_and(|first| {
            first == "/modo"
                || super::commands::COMMANDS
                    .iter()
                    .any(|c| first.strip_prefix('/') == Some(c.name))
        });
        if text.is_empty() || (text.starts_with('/') && (self != Self::Qa || explicit_command)) {
            return text.into();
        }
        match self {
            Self::Feature => format!("/plan {text}"),
            Self::Qa => format!("/qa {text}"),
            Self::Conversation => text.into(),
        }
    }
}
