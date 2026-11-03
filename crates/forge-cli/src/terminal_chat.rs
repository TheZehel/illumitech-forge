//! Interactive Forge terminal frontend. Uses the same guarded native bridge as the web UI.
mod activity;
mod activity_panel;
mod commands;
mod editor;
mod markdown;
mod transcript;
mod ui;
mod workflows;
use crate::workbench::{Action, App, Session, View, approval_choices};
use anyhow::{Result, ensure};
use commands::Panel;
use crossterm::{
    event::{
        self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use serde_json::{Value, json};
use std::{
    io::{IsTerminal, stdout},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::task::JoinHandle;

#[derive(Default)]
struct Chat {
    selected: String,
    editor: editor::Editor,
    notice: String,
    busy: String,
    scroll_back: u16,
    tick: usize,
    panel: Option<Panel>,
    panel_selection: usize,
    panel_scroll: u16,
    palette_dismissed: bool,
    history_query: String,
    palette_selection: usize,
    activities: bool,
    activity_panel: activity_panel::State,
    transcript: transcript::Cache,
    active_started: Option<std::time::Instant>,
    choosing: Option<Value>,
    workflow: Value,
    workflow_id: String,
    workflow_digest: String,
}
impl Chat {
    fn history_matches(&self) -> Vec<String> {
        let query = self.history_query.to_lowercase();
        self.transcript
            .history
            .iter()
            .rev()
            .filter(|s| s.to_lowercase().contains(&query))
            .cloned()
            .collect()
    }
    fn open_panel(&mut self, panel: Panel) {
        self.panel = Some(panel);
        if panel == Panel::Activities {
            self.activity_panel.focus();
        }
        self.panel_selection = 0;
        self.panel_scroll = 0;
        if panel == Panel::History {
            self.history_query.clear();
        }
    }
}
struct Pending {
    handle: JoinHandle<Result<Value>>,
    route: String,
    draft: String,
}
struct Restore;
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = execute!(stdout(), DisableBracketedPaste);
        ratatui::restore();
    }
}
/// Strip control characters from all untrusted text; an LLM cannot emit terminal escapes.
fn display(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}
fn action(thread: &str) -> Result<Action> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes)?;
    Ok(serde_json::from_value(json!({
        "operation_id": format!("terminal-{}", bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()),
        "thread_id": (!thread.is_empty()).then_some(thread)
    }))?)
}
fn start(app: &Arc<App>, route: &str, mut payload: Action, draft: String) -> Pending {
    let app = app.clone();
    let name = route.to_owned();
    Pending {
        handle: tokio::spawn(async move {
            if name == "turn" && payload.thread_id.is_none() {
                let created = app.action("sessions", action("")?).await?;
                payload.thread_id = created["thread_id"].as_str().map(str::to_owned);
            }
            let thread = payload.thread_id.clone();
            let mut result = app.action(&name, payload).await?;
            if name == "turn" {
                result["thread_id"] = json!(thread);
            }
            Ok(result)
        }),
        route: route.into(),
        draft,
    }
}
fn busy_label(route: &str) -> &'static str {
    match route {
        "connect" | "reconnect" => "Conectando ao cliente…",
        "resume" => "Retomando conversa…",
        "sessions" => "Criando conversa…",
        "interrupt" => "Interrompendo resposta…",
        "approval" => "Enviando sua decisão…",
        _ => "Enviando mensagem…",
    }
}
fn decision_label(value: &Value) -> String {
    match value.as_str() {
        Some("accept") => "Permitir uma vez".into(),
        Some("acceptForSession") => "Permitir nesta sessão".into(),
        Some("decline") => "Recusar".into(),
        Some("cancel") => "Recusar e interromper".into(),
        _ => display(&value.to_string()),
    }
}

pub async fn run(root: &Path, state: &Path, executable: Option<PathBuf>) -> Result<Value> {
    ensure!(
        std::io::stdin().is_terminal() && stdout().is_terminal(),
        "chat requires an interactive terminal; use workbench for the browser UI"
    );
    let app = App::new(root, state, executable, None)?;
    let _restore = Restore;
    let mut terminal = ratatui::try_init()?;
    execute!(stdout(), EnableBracketedPaste)?;
    let mut chat = Chat {
        busy: "connect".into(),
        ..Default::default()
    };
    let connected_app = app.clone();
    let mut pending = Some(Pending {
        handle: tokio::spawn(async move {
            connected_app.connect().await?;
            Ok(json!({"connected":true}))
        }),
        route: "connect".into(),
        draft: String::new(),
    });
    let result = tokio::select! {
        result = run_loop(&app, &mut terminal, &mut chat, &mut pending) => result,
        _ = shutdown_signal() => Ok(()),
    };
    if let Some(p) = pending {
        p.handle.abort();
        let _ = p.handle.await;
    }
    app.stop_forge().await;
    if let Some(n) = app.native.write().await.take() {
        n.shutdown().await;
    }
    result?;
    Ok(json!({"status":"closed","interface":"terminal","replayed":false}))
}
async fn shutdown_signal() {
    #[cfg(unix)]
    if let Ok(mut signal) =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
    {
        tokio::select! { _ = signal.recv() => {}, _ = tokio::signal::ctrl_c() => {} }
        return;
    }
    let _ = tokio::signal::ctrl_c().await;
}
async fn run_loop(
    app: &Arc<App>,
    terminal: &mut ratatui::DefaultTerminal,
    chat: &mut Chat,
    pending: &mut Option<Pending>,
) -> Result<()> {
    loop {
        if pending.as_ref().is_some_and(|p| p.handle.is_finished()) {
            let p = pending.take().unwrap();
            match p.handle.await? {
                Ok(v) => {
                    if p.route == "sessions" || p.route == "turn" {
                        chat.selected = v["thread_id"].as_str().unwrap_or("").into();
                        chat.panel = None;
                    }
                    if p.route == "turn" && chat.editor.text == p.draft {
                        chat.editor.clear();
                    }
                    chat.notice.clear();
                    if p.route.starts_with("workflow/") {
                        if let Some(id) = v["workflow_id"].as_str() {
                            chat.workflow_id = id.into();
                        }
                        if let Some(digest) = v["scope_digest"].as_str() {
                            chat.workflow_digest = digest.into();
                        }
                        chat.workflow = v;
                        chat.open_panel(Panel::Workflow);
                    }
                }
                Err(e) => chat.notice = format!("{e}. Nenhuma ação reenviada automaticamente."),
            }
            chat.busy.clear();
        }
        let view = app.view.lock().await;
        if chat.selected.is_empty() {
            chat.selected = view.sessions.keys().next().cloned().unwrap_or_default();
        }
        terminal.draw(|f| chat.draw(f, &view, &app.root))?;
        let connected = view.connected && view.account["authenticated"].as_bool() == Some(true);
        let session = view.sessions.get(&chat.selected);
        let active = session.is_some_and(|s| s.active_turn.is_some());
        let loaded = session.is_some_and(|s| s.loaded);
        let sessions = view.sessions.keys().cloned().collect::<Vec<_>>();
        drop(view);
        chat.tick = chat.tick.wrapping_add(1);
        // poll/read are called from this same thread only and never block a native RPC.
        if !event::poll(Duration::ZERO)? {
            tokio::time::sleep(Duration::from_millis(80)).await;
            continue;
        }
        match event::read()? {
            Event::Paste(text) => {
                if chat.panel == Some(Panel::History) {
                    let clean = display(&text);
                    if chat.history_query.len() + clean.len() <= 4096 {
                        chat.history_query.push_str(&clean);
                        chat.panel_selection = 0;
                    }
                } else if chat.panel.is_none() {
                    chat.editor.insert(&text);
                    chat.palette_dismissed = false;
                    chat.palette_selection = 0;
                }
            }
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                if ctrl && key.code == KeyCode::Char('q') {
                    return Ok(());
                }
                if key.code == KeyCode::F(1)
                    || (key.code == KeyCode::Char('?')
                        && chat.editor.text.is_empty()
                        && chat.panel.is_none())
                {
                    if chat.panel == Some(Panel::Help) {
                        chat.panel = None;
                    } else {
                        chat.open_panel(Panel::Help);
                    }
                    continue;
                }
                if ctrl && key.code == KeyCode::Char('l') {
                    terminal.clear()?;
                    continue;
                }
                if ctrl && key.code == KeyCode::Char('r') {
                    chat.open_panel(Panel::History);
                    continue;
                }
                if ctrl && key.code == KeyCode::Char('o') {
                    if chat.panel == Some(Panel::Activities) {
                        chat.activity_panel.toggle();
                    } else {
                        chat.activities = !chat.activities;
                    }
                    continue;
                }
                if key.code == KeyCode::F(2) {
                    if chat.panel == Some(Panel::Activities) {
                        chat.panel = None;
                    } else {
                        chat.open_panel(Panel::Activities);
                    }
                    continue;
                }
                if key.code == KeyCode::PageUp || key.code == KeyCode::PageDown {
                    if chat.panel.is_some() {
                        chat.panel_scroll = if key.code == KeyCode::PageDown {
                            chat.panel_scroll.saturating_add(8)
                        } else {
                            chat.panel_scroll.saturating_sub(8)
                        };
                    } else {
                        chat.scroll_back = if key.code == KeyCode::PageUp {
                            chat.scroll_back.saturating_add(10)
                        } else {
                            chat.scroll_back.saturating_sub(10)
                        };
                    }
                    continue;
                }
                if let Some(panel) = chat.panel {
                    if key.code == KeyCode::Esc {
                        chat.panel = None;
                        continue;
                    }
                    if panel == Panel::Activities {
                        match key.code {
                            KeyCode::Up => chat.activity_panel.move_by(-1),
                            KeyCode::Down => chat.activity_panel.move_by(1),
                            KeyCode::Enter | KeyCode::Char(' ') => chat.activity_panel.toggle(),
                            KeyCode::Left => chat.activity_panel.set_open(false),
                            KeyCode::Right => chat.activity_panel.set_open(true),
                            KeyCode::Home => chat.activity_panel.move_by(isize::MIN),
                            KeyCode::End => chat.activity_panel.move_by(isize::MAX),
                            _ => {}
                        }
                        continue;
                    }
                    let count = if panel == Panel::Sessions {
                        sessions.len()
                    } else {
                        chat.history_matches().len()
                    };
                    if key.code == KeyCode::Up {
                        chat.panel_selection = chat.panel_selection.saturating_sub(1);
                        continue;
                    }
                    if key.code == KeyCode::Down {
                        chat.panel_selection =
                            (chat.panel_selection + 1).min(count.saturating_sub(1));
                        continue;
                    }
                    if panel == Panel::History {
                        match key.code {
                            KeyCode::Enter => {
                                if let Some(text) = chat.history_matches().get(chat.panel_selection)
                                {
                                    chat.editor.set(text.clone());
                                    chat.panel = None;
                                }
                                continue;
                            }
                            KeyCode::Backspace => {
                                chat.history_query.pop();
                                chat.panel_selection = 0;
                            }
                            KeyCode::Char('u') if ctrl => {
                                chat.history_query.clear();
                                chat.panel_selection = 0;
                            }
                            KeyCode::Char(c)
                                if !ctrl
                                    && !key.modifiers.contains(KeyModifiers::ALT)
                                    && chat.history_query.len() < 4096 =>
                            {
                                chat.history_query.push(c);
                                chat.panel_selection = 0;
                            }
                            _ => {}
                        }
                        continue;
                    }
                    if panel == Panel::Sessions && key.code == KeyCode::Enter {
                        if active || pending.is_some() {
                            chat.notice =
                                "Aguarde ou interrompa a resposta antes de retomar outra conversa."
                                    .into();
                            continue;
                        }
                        if count == 0 {
                            continue;
                        }
                        chat.editor
                            .set(format!("/resume {}", chat.panel_selection + 1));
                        chat.panel = None;
                    } else {
                        continue;
                    }
                }
                if key.code == KeyCode::Enter
                    && key
                        .modifiers
                        .intersects(KeyModifiers::ALT | KeyModifiers::SHIFT)
                {
                    chat.editor.insert("\n");
                    continue;
                }
                let matches = commands::matching(&chat.editor.text);
                if !matches.is_empty() && !chat.palette_dismissed {
                    match key.code {
                        KeyCode::Up => {
                            chat.palette_selection = chat.palette_selection.saturating_sub(1);
                            continue;
                        }
                        KeyCode::Down => {
                            chat.palette_selection =
                                (chat.palette_selection + 1).min(matches.len() - 1);
                            continue;
                        }
                        KeyCode::Tab => {
                            chat.editor.set(format!(
                                "/{}",
                                matches[chat.palette_selection.min(matches.len() - 1)].name
                            ));
                            chat.palette_selection = 0;
                            continue;
                        }
                        KeyCode::Enter => {
                            chat.editor.set(format!(
                                "/{}",
                                matches[chat.palette_selection.min(matches.len() - 1)].name
                            ));
                        }
                        KeyCode::Esc => {
                            chat.palette_dismissed = true;
                            continue;
                        }
                        _ => {}
                    }
                }
                if key.code == KeyCode::Up || key.code == KeyCode::Down {
                    if !chat.editor.text.contains('\n') {
                        chat.editor
                            .history(&chat.transcript.history, key.code == KeyCode::Up);
                    } else {
                        chat.editor.edit(key);
                    }
                    continue;
                }
                // Local commands remain available while a native request is pending.
                if key.code == KeyCode::Enter {
                    match chat.editor.text.trim() {
                        "/help" => {
                            chat.editor.clear();
                            chat.open_panel(Panel::Help);
                            continue;
                        }
                        "/status" => {
                            chat.editor.clear();
                            chat.open_panel(Panel::Status);
                            continue;
                        }
                        "/history" => {
                            chat.editor.clear();
                            chat.open_panel(Panel::History);
                            continue;
                        }
                        "/activities" => {
                            chat.editor.clear();
                            chat.open_panel(Panel::Activities);
                            continue;
                        }
                        "/sessions" => {
                            chat.editor.clear();
                            chat.open_panel(Panel::Sessions);
                            continue;
                        }
                        "/quit" | "/exit" => return Ok(()),
                        _ => {}
                    }
                }
                if pending.is_some() {
                    if key.code != KeyCode::Enter {
                        chat.editor.edit(key);
                    }
                    continue;
                }
                if key.code == KeyCode::Enter {
                    let text = chat.editor.text.trim().to_owned();
                    match workflows::pending(app, chat, &text) {
                        Ok(Some(p)) => {
                            chat.busy = p.route.clone();
                            chat.editor.clear();
                            *pending = Some(p);
                            continue;
                        }
                        Err(e) => {
                            chat.notice = e.to_string();
                            continue;
                        }
                        Ok(None) => {}
                    }
                }
                let mut route = String::new();
                let mut payload = action(&chat.selected)?;
                if (key.code == KeyCode::Esc || ctrl && key.code == KeyCode::Char('c')) && active {
                    route = "interrupt".into();
                } else if ctrl && key.code == KeyCode::Char('c') {
                    chat.editor.clear_recoverable();
                } else if key.modifiers.contains(KeyModifiers::ALT)
                    && let KeyCode::Char(c @ '1'..='9') = key.code
                    && let Some(a) = &chat.choosing
                {
                    if let Some(choice) = approval_choices(a).get((c as u8 - b'1') as usize) {
                        payload.request_id = Some(a["id"].clone());
                        payload.decision = Some(choice.clone());
                        route = "approval".into();
                    }
                } else if key.code == KeyCode::Enter && !chat.editor.text.trim().is_empty() {
                    let command = chat.editor.text.trim();
                    if command == "/quit" {
                        return Ok(());
                    }
                    if command == "/new" {
                        route = "sessions".into();
                    } else if command == "/reconnect" {
                        route = "reconnect".into();
                    } else if command == "/resume" || command.starts_with("/resume ") {
                        if active {
                            chat.notice =
                                "Interrompa o turno antes de retomar outra conversa.".into();
                            continue;
                        }
                        let id = command
                            .split_whitespace()
                            .nth(1)
                            .and_then(|n| n.parse::<usize>().ok())
                            .and_then(|i| i.checked_sub(1))
                            .and_then(|i| sessions.get(i));
                        if command != "/resume" && id.is_none() {
                            chat.notice = "Sessão inválida. Use /sessions para listar.".into();
                            continue;
                        }
                        if let Some(id) = id {
                            chat.selected = id.clone();
                            payload.thread_id = Some(id.clone());
                        }
                        route = "resume".into();
                        chat.panel = None;
                    } else if command.starts_with('/') {
                        chat.notice = "Comando desconhecido. Use /help.".into();
                        continue;
                    } else if !connected {
                        chat.notice = "Entre pelo cliente nativo e use /reconnect.".into();
                        continue;
                    } else if active {
                        chat.notice =
                            "Aguarde a resposta ou use Esc para interromper. Rascunho preservado."
                                .into();
                        continue;
                    } else if !loaded && !chat.selected.is_empty() {
                        chat.notice =
                            "Use /new para começar ou /resume para retomar uma sessão salva."
                                .into();
                        continue;
                    } else {
                        route = "turn".into();
                        payload.text = Some(chat.editor.text.clone());
                        chat.scroll_back = 0;
                        chat.panel = None;
                    }
                } else {
                    let before = chat.editor.text.clone();
                    chat.editor.edit(key);
                    if chat.editor.text != before {
                        chat.palette_dismissed = false;
                        chat.palette_selection = 0;
                    }
                }
                if !route.is_empty() {
                    let draft = chat.editor.text.clone();
                    if route != "turn" {
                        chat.editor.clear();
                    }
                    chat.busy = route.clone();
                    *pending = Some(start(app, &route, payload, draft));
                }
            }
            _ => {}
        }
    }
}

/// Choose a project before opening any native session or operational database.
pub fn choose_project(projects: &[PathBuf]) -> Result<PathBuf> {
    use ratatui::widgets::{List, ListItem, ListState};
    ensure!(!projects.is_empty(), "no projects available");
    let _restore = Restore;
    let mut terminal = ratatui::try_init()?;
    let mut selected = 0;
    loop {
        terminal.draw(|f| {
            let rows = projects
                .iter()
                .map(|p| {
                    ListItem::new(display(
                        &p.file_name().unwrap_or_default().to_string_lossy(),
                    ))
                })
                .collect::<Vec<_>>();
            let branded = f.area().width >= 55 && f.area().height >= 24;
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(if branded { 9 } else { 4 }),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ])
                .split(f.area());
            f.render_widget(
                Paragraph::new(if branded {
                    let mut lines = ui::banner();
                    lines.push(Line::default());
                    lines.push(Line::from("Escolha um projeto para conversar."));
                    lines
                } else {
                    vec![
                        Line::from("ILLUMITECH FORGE"),
                        Line::from("Escolha um projeto para conversar."),
                    ]
                })
                .style(Style::default().fg(Color::Rgb(255, 154, 82))),
                chunks[0],
            );
            let mut state = ListState::default().with_selected(Some(selected));
            f.render_stateful_widget(
                List::new(rows)
                    .highlight_symbol("› ")
                    .highlight_style(Style::default().fg(Color::Rgb(255, 154, 82)).bold()),
                chunks[1],
                &mut state,
            );
            f.render_widget(
                Paragraph::new(format!(
                    "{}\n↑/↓ selecionam · Enter abre · Esc cancela",
                    display(&projects[selected].to_string_lossy())
                ))
                .wrap(Wrap { trim: false }),
                chunks[2],
            );
        })?;
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Up => selected = selected.checked_sub(1).unwrap_or(projects.len() - 1),
                KeyCode::Down => selected = (selected + 1) % projects.len(),
                KeyCode::Enter => return Ok(projects[selected].clone()),
                KeyCode::Esc => anyhow::bail!("Seleção cancelada; nenhum cliente foi iniciado."),
                KeyCode::Char('c' | 'q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    anyhow::bail!("Seleção cancelada; nenhum cliente foi iniciado.")
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn narrow_render_never_panics_and_keeps_offline_state_visible() {
        for (w, h) in [(80, 24), (42, 16), (20, 5)] {
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
            let mut chat = Chat::default();
            terminal
                .draw(|f| chat.draw(f, &View::default(), Path::new("/project")))
                .unwrap();
        }
        assert_eq!(
            display("safe\x1b]52;copy\x07\r\ntext"),
            "safe]52;copy\ntext"
        );
    }
}
