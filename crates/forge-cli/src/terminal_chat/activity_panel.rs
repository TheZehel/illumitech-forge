//! Keyboard-only, per-item disclosure. This view cannot execute tools or approve gates.
use super::{Line, Session, activity, display, markdown};
use ratatui::{
    style::{Color, Style},
    widgets::{Paragraph, Wrap},
};
use std::collections::BTreeSet;

struct Node {
    key: String,
    title: String,
    body: String,
    children: Vec<Node>,
}
fn bounded(text: &str) -> String {
    let end = text.floor_char_boundary(text.len().min(24000));
    let value = display(&text[..end]);
    if end < text.len() {
        format!("{value}\n… Exibição limitada a 24000 bytes.")
    } else {
        value
    }
}
fn state_label(status: &str) -> &str {
    match status {
        "completed" => " · concluída",
        "failed" => " · falhou",
        "inProgress" => " · em andamento",
        "interrupted" => " · interrompida",
        "declined" => " · recusada",
        _ => "",
    }
}
#[derive(Default)]
pub(super) struct State {
    session: String,
    events: std::collections::VecDeque<serde_json::Value>,
    nodes: Vec<Node>,
    selected: String,
    open: BTreeSet<String>,
    reveal: bool,
    viewport: (u16, u16),
}
impl State {
    pub fn focus(&mut self) {
        self.reveal = true;
    }
    pub fn update(&mut self, session: Option<&Session>) {
        let id = session.map(|s| s.id.as_str()).unwrap_or("");
        if self.session != id {
            *self = Self {
                session: id.into(),
                ..Self::default()
            };
        }
        let Some(session) = session else {
            self.nodes.clear();
            self.open.clear();
            self.selected.clear();
            return;
        };
        if self.events == session.events {
            return;
        }
        self.events = session.events.clone();
        let mut items = activity::items(&session.events);
        let mut seen = BTreeSet::new();
        self.nodes = session
            .events
            .iter()
            .filter_map(|event| {
                let p = &event["params"];
                let id = p["item"]["id"].as_str().or(p["itemId"].as_str())?;
                if !seen.insert(id.to_owned()) {
                    return None;
                }
                let item = items.remove(id)?;
                if matches!(
                    item["type"].as_str(),
                    Some("userMessage" | "agentMessage" | "reasoning")
                ) {
                    return None;
                }
                let entry = activity::entry(&item);
                let (tag, body) = entry.split_once('\n').unwrap_or((&entry, ""));
                let (title, status) = tag.split_once('|').unwrap_or((tag, ""));
                let mut node = Node {
                    key: format!("item:{id}"),
                    title: display(&format!("{title}{}", state_label(status))),
                    body: bounded(body),
                    children: Vec::new(),
                };
                if item["type"] == "fileChange" {
                    node.body = if item["forgeDisplayTruncated"] == true {
                        "Exibição parcial; conteúdo e contagens podem estar incompletos.\n".into()
                    } else {
                        String::new()
                    };
                    if let Some(output) = item["aggregatedOutput"].as_str() {
                        node.body.push_str(&format!("Saída\n{}", bounded(output)));
                    }
                    if let Some(files) = item["changes"].as_array() {
                        for (index, file) in files.iter().enumerate().filter(|(_, f)| f.is_object())
                        {
                            let path = file["path"]
                                .as_str()
                                .unwrap_or("Arquivo sem caminho informado");
                            let diff = file["diff"].as_str().unwrap_or("");
                            let counts = activity::counts(diff)
                                .map(|(a, r)| format!(" (+{a} −{r})"))
                                .unwrap_or_default();
                            node.children.push(Node {
                                key: format!("item:{id}/file:{index}"),
                                title: display(&format!("{path}{counts}")).replace('\n', " "),
                                body: if diff.is_empty() {
                                    "Diff não fornecido pelo cliente.".into()
                                } else {
                                    bounded(diff)
                                },
                                children: Vec::new(),
                            });
                        }
                    }
                }
                Some(node)
            })
            .collect();
        let all = self
            .nodes
            .iter()
            .flat_map(|n| std::iter::once(&n.key).chain(n.children.iter().map(|c| &c.key)))
            .cloned()
            .collect::<BTreeSet<_>>();
        self.open.retain(|key| all.contains(key));
        if !all.contains(&self.selected) {
            self.selected = self.nodes.last().map(|n| n.key.clone()).unwrap_or_default();
            self.reveal = true;
        }
    }
    fn visible(&self) -> Vec<&Node> {
        self.nodes
            .iter()
            .flat_map(|n| {
                std::iter::once(n).chain(n.children.iter().filter(|_| self.open.contains(&n.key)))
            })
            .collect()
    }
    pub fn move_by(&mut self, delta: isize) {
        let keys = self
            .visible()
            .iter()
            .map(|n| n.key.clone())
            .collect::<Vec<_>>();
        if keys.is_empty() {
            return;
        }
        let i = keys
            .iter()
            .position(|key| key == &self.selected)
            .unwrap_or(0);
        self.selected = keys[i.saturating_add_signed(delta).min(keys.len() - 1)].clone();
        self.reveal = true;
    }
    pub fn toggle(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        if !self.open.remove(&self.selected) {
            self.open.insert(self.selected.clone());
        }
        self.reveal = true;
    }
    pub fn set_open(&mut self, open: bool) {
        if self.selected.is_empty() {
            return;
        }
        if open {
            self.open.insert(self.selected.clone());
        } else {
            self.open.remove(&self.selected);
        }
        self.reveal = true;
    }
    pub fn lines(&mut self, width: u16, height: u16, scroll: &mut u16) -> Vec<Line<'static>> {
        if self.viewport != (width, height) {
            self.viewport = (width, height);
            self.reveal = true;
        }
        let mut lines = if height < 12 {
            vec![Line::from("↑/↓ · Enter abre/fecha · Esc volta")]
        } else {
            vec![
                Line::from(
                    "↑/↓ selecionam · Enter/Espaço mostram ou ocultam · ←/→ recolhem/expandem",
                ),
                Line::from("PgUp/PgDn percorrem os detalhes · Esc volta ao rascunho"),
                Line::from("Atividade concluída não comprova aceite dos gates."),
                Line::default(),
            ]
        };
        let mut anchor = 0;
        let mut selected_height = 1;
        for node in self.visible() {
            let selected = self.selected == node.key;
            if selected {
                anchor = lines
                    .iter()
                    .map(|line| {
                        Paragraph::new(line.clone())
                            .wrap(Wrap { trim: false })
                            .line_count(width.max(1))
                    })
                    .sum::<usize>();
            }
            let open = self.open.contains(&node.key);
            let indent = if node.key.contains("/file:") {
                "    "
            } else {
                ""
            };
            let header = Line::styled(
                format!(
                    "{}{indent}{} {} · {}",
                    if selected { "› " } else { "  " },
                    if open { "▼" } else { "▶" },
                    node.title,
                    if open {
                        "Ocultar detalhes"
                    } else {
                        "Mostrar detalhes"
                    }
                ),
                Style::default()
                    .fg(if selected {
                        markdown::ORANGE
                    } else {
                        markdown::TEAL
                    })
                    .bold(),
            );
            if selected {
                selected_height = Paragraph::new(header.clone())
                    .wrap(Wrap { trim: false })
                    .line_count(width.max(1));
            }
            lines.push(header);
            if open {
                lines.extend(node.body.lines().map(|line| {
                    Line::styled(
                        format!("    {line}"),
                        Style::default().fg(if line.starts_with('+') {
                            Color::Green
                        } else if line.starts_with('-') {
                            Color::LightRed
                        } else {
                            Color::Gray
                        }),
                    )
                }));
                lines.push(Line::default());
            }
        }
        if self.nodes.is_empty() {
            lines.push(Line::from("Nenhuma atividade informada nesta sessão."));
        }
        let height = height.max(1) as usize;
        if self.reveal
            && (anchor < *scroll as usize
                || anchor + selected_height.min(height) > *scroll as usize + height)
        {
            *scroll = anchor.min(u16::MAX as usize) as u16;
        }
        self.reveal = false;
        let max = Paragraph::new(lines.clone())
            .wrap(Wrap { trim: false })
            .line_count(width.max(1))
            .saturating_sub(height)
            .min(u16::MAX as usize) as u16;
        *scroll = (*scroll).min(max);
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn compact_resize_and_reopen_reveal_the_selected_activity() {
        let mut state = State::default();
        state.update(Some(&session()));
        let mut scroll = 0;
        state.lines(140, 30, &mut scroll);
        let lines = state.lines(40, 7, &mut scroll);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 7)).unwrap();
        terminal
            .draw(|f| {
                f.render_widget(
                    Paragraph::new(lines)
                        .wrap(Wrap { trim: false })
                        .scroll((scroll, 0)),
                    f.area(),
                )
            })
            .unwrap();
        let visible = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(visible.contains("Alterações"));
        scroll = 0;
        state.focus();
        state.lines(20, 4, &mut scroll);
        assert!(scroll > 0);
    }
    fn session() -> Session {
        Session { id: "one".into(), events: [json!({"method":"item/started","params":{"item":{"id":"tool","type":"mcpToolCall","server":"logs","tool":"query","arguments":{"limit":1}}}}),json!({"method":"item/completed","params":{"item":{"id":"files","type":"fileChange","changes":[{"path":"src/a.rs","diff":"@@ -1 +1 @@\n-old\n+NEW_DIFF\n"}]}}})].into(), ..Default::default() }
    }
    fn text(s: &mut State) -> String {
        s.lines(100, 25, &mut 0)
            .iter()
            .map(ToString::to_string)
            .collect::<String>()
    }
    #[test]
    fn disclosure_is_per_activity_and_file_and_survives_updates() {
        let mut session = session();
        let mut s = State::default();
        s.update(Some(&session));
        assert!(!text(&mut s).contains("NEW_DIFF"));
        s.toggle();
        assert!(text(&mut s).contains("src/a.rs"));
        assert!(!text(&mut s).contains("NEW_DIFF"));
        s.move_by(1);
        s.toggle();
        assert!(text(&mut s).contains("NEW_DIFF"));
        session.events.push_back(json!({"method":"item/completed","params":{"item":{"id":"tool","type":"mcpToolCall","status":"failed","error":{"message":"TOOL_ERROR"}}}}));
        s.update(Some(&session));
        assert!(text(&mut s).contains("NEW_DIFF"));
        assert!(!text(&mut s).contains("TOOL_ERROR"));
        s.move_by(-2);
        s.toggle();
        assert!(text(&mut s).contains("TOOL_ERROR"));
        assert!(text(&mut s).contains("falhou"));
        session.id = "other".into();
        s.update(Some(&session));
        assert!(!text(&mut s).contains("NEW_DIFF"));
        assert!(!text(&mut s).contains("TOOL_ERROR"));
        s.update(None);
        assert!(text(&mut s).contains("Nenhuma atividade"));
    }
    #[test]
    fn scrolling_selection_stays_reachable_on_narrow_views() {
        let mut s = State::default();
        s.update(Some(&session()));
        s.toggle();
        s.move_by(1);
        s.toggle();
        let mut scroll = 0;
        let lines = s.lines(20, 4, &mut scroll);
        assert!(scroll > 0);
        assert!(lines.iter().any(|l| l.to_string().contains("src/a.rs")));
        s.set_open(false);
        assert!(!text(&mut s).contains("NEW_DIFF"));
    }
}
