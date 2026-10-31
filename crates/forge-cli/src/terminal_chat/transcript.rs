use super::{Line, Session, display, markdown};
use ratatui::style::{Color, Style};
use serde_json::Value;
use std::collections::VecDeque;
fn entries(session: Option<&Session>) -> Vec<(&'static str, String)> {
    let mut lines = Vec::new();
    let mut agents = std::collections::BTreeMap::<String, String>::new();
    let Some(session) = session else {
        return Vec::new();
    };
    // Collapse deltas into their item, retaining chronological placement and final text.
    for e in &session.events {
        let p = &e["params"];
        let method = e["method"].as_str().unwrap_or("");
        if method == "item/agentMessage/delta" {
            agents
                .entry(p["itemId"].as_str().unwrap_or("").into())
                .or_default()
                .push_str(p["delta"].as_str().unwrap_or(""));
        } else if method == "item/completed" && p["item"]["type"] == "agentMessage" {
            agents.insert(
                p["item"]["id"].as_str().unwrap_or("").into(),
                p["item"]["text"].as_str().unwrap_or("").into(),
            );
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut echo = None;
    for e in &session.events {
        let p = &e["params"];
        let method = e["method"].as_str().unwrap_or("");
        let item = &p["item"];
        let kind = item["type"].as_str().unwrap_or("");
        let (label, text) = if method == "bridge/user" {
            let text = p["text"].as_str().unwrap_or("").to_owned();
            echo = Some(text.clone());
            ("Você", text)
        } else if method == "item/completed" && kind == "userMessage" {
            let text = item["text"].as_str().map(str::to_owned).unwrap_or_else(|| {
                item["content"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v["text"].as_str())
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default()
            });
            if echo.as_ref() == Some(&text) {
                echo = None;
                continue;
            }
            ("Você", text)
        } else if method == "item/agentMessage/delta"
            || method == "item/completed" && kind == "agentMessage"
        {
            let id = p["itemId"].as_str().or(item["id"].as_str()).unwrap_or("");
            if !seen.insert(id.to_owned()) {
                continue;
            }
            ("Forge", agents.get(id).cloned().unwrap_or_default())
        } else if method == "turn/completed" {
            echo = None;
            (
                "Estado",
                match p["turn"]["status"].as_str().unwrap_or("") {
                    "completed" => "Resposta concluída",
                    "interrupted" => "Resposta interrompida",
                    "failed" => "Não foi possível concluir a resposta",
                    _ => "Resposta encerrada",
                }
                .into(),
            )
        } else if method == "error" || method == "bridge/truncated" {
            (
                "Atenção",
                p["error"]["message"]
                    .as_str()
                    .or(p["reason"].as_str())
                    .unwrap_or("Consulte os detalhes do cliente.")
                    .into(),
            )
        } else if method == "bridge/decision" {
            ("Decisão enviada", super::decision_label(&p["decision"]))
        } else if method == "item/completed" && !kind.is_empty() {
            (
                "Atividade",
                format!(
                    "{}|{}\n{}",
                    kind,
                    item["status"].as_str().unwrap_or(""),
                    item["aggregatedOutput"]
                        .as_str()
                        .or(item["command"].as_str())
                        .unwrap_or("")
                ),
            )
        } else {
            continue;
        };
        if label == "Atividade"
            && text.starts_with("reasoning|")
            && text
                .split_once('\n')
                .is_some_and(|(_, output)| output.trim().is_empty())
        {
            continue;
        }
        lines.push((label, text));
        if method == "turn/completed" && !p["turn"]["error"].is_null() {
            lines.push((
                "Atenção",
                p["turn"]["error"]["message"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| p["turn"]["error"].to_string()),
            ));
        }
    }
    lines
}

fn render(session: Option<&Session>, details: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (label, text) in entries(session) {
        if label == "Estado" {
            lines.push(Line::styled(text, Style::default().fg(Color::DarkGray)));
            lines.push(Line::default());
            continue;
        }
        if label == "Atividade" {
            let (tag, output) = text.split_once('\n').unwrap_or((&text, ""));
            let (kind, status) = tag.split_once('|').unwrap_or((tag, ""));
            let state = match status {
                "failed" => " · falhou",
                "completed" => " · concluída",
                "interrupted" => " · interrompida",
                "declined" => " · recusada",
                "inProgress" => " · em andamento",
                _ => "",
            };
            let title = match kind {
                "commandExecution" => "Ferramenta",
                "fileChange" => "Mudanças em arquivos",
                "webSearch" => "Pesquisa",
                "reasoning" => "Raciocínio disponibilizado pelo cliente",
                _ => "Atividade do cliente",
            };
            lines.push(Line::styled(
                format!(
                    "  › {title}{state}{}",
                    if details {
                        ""
                    } else {
                        " · Ctrl+O para detalhes"
                    }
                ),
                Style::default().fg(markdown::TEAL),
            ));
            if details {
                lines.extend(
                    display(output)
                        .lines()
                        .map(|s| Line::from(format!("    {s}"))),
                );
            }
            continue;
        }
        lines.push(Line::styled(
            label.to_owned(),
            Style::default()
                .fg(if label == "Você" {
                    markdown::TEAL
                } else {
                    markdown::ORANGE
                })
                .bold(),
        ));
        if label == "Forge" {
            lines.extend(markdown::render(&text));
        } else {
            lines.extend(display(&text).lines().map(|s| Line::from(s.to_owned())));
        }
        lines.push(Line::default());
    }
    lines
}
#[derive(Default)]
pub(super) struct Cache {
    id: String,
    events: VecDeque<Value>,
    details: bool,
    pub lines: Vec<Line<'static>>,
    pub history: Vec<String>,
}
impl Cache {
    pub fn update(&mut self, session: Option<&Session>, details: bool) {
        let id = session.map(|s| s.id.as_str()).unwrap_or("");
        let changed = self.id != id
            || session.is_some_and(|s| s.events != self.events)
            || (session.is_none() && !self.events.is_empty());
        if changed || self.details != details {
            self.id = id.into();
            self.events = session.map(|s| s.events.clone()).unwrap_or_default();
            self.details = details;
            self.lines = render(session, details);
            let mut history = entries(session)
                .into_iter()
                .filter(|(label, _)| *label == "Você")
                .rev()
                .map(|(_, text)| text)
                .take(50)
                .collect::<Vec<_>>();
            let mut bytes = 0;
            history.retain(|s| {
                bytes += s.len();
                bytes <= 256 * 1024
            });
            history.reverse();
            self.history = history;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn streaming_replaces_partial_and_activity_details_are_opt_in() {
        let session=Session{events:[json!({"method":"bridge/user","params":{"text":"pedido"}}),json!({"method":"item/agentMessage/delta","params":{"itemId":"a","delta":"parcial"}}),json!({"method":"item/completed","params":{"item":{"id":"a","type":"agentMessage","text":"**Resposta** completa"}}}),json!({"method":"item/completed","params":{"item":{"id":"tool","type":"commandExecution","aggregatedOutput":"SAIDA_PRIVADA_DO_TESTE"}}})].into(),..Default::default()};
        let mut cache = Cache::default();
        cache.update(Some(&session), false);
        let text = cache
            .lines
            .iter()
            .map(ToString::to_string)
            .collect::<String>();
        assert!(text.contains("Resposta completa"));
        assert!(!text.contains("parcial"));
        assert!(!text.contains("SAIDA_PRIVADA_DO_TESTE"));
        assert_eq!(cache.history, vec!["pedido"]);
        cache.update(Some(&session), true);
        assert!(
            cache
                .lines
                .iter()
                .map(ToString::to_string)
                .collect::<String>()
                .contains("SAIDA_PRIVADA_DO_TESTE")
        );
        cache.update(None, false);
        assert!(cache.history.is_empty());
    }
}
