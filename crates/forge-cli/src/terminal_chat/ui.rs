use super::*;
use commands::{COMMANDS, matching};
use markdown::{ORANGE, TEAL};
use ratatui::{
    layout::Rect,
    text::Span,
    widgets::{BorderType, List, ListItem, ListState},
};

pub(super) const ASCII: &[&str] = &[
    r" _____ ___  ____   ____ _____",
    r"|  ___/ _ \|  _ \ / ___| ____|",
    r"| |_ | | | | |_) | |  _|  _|",
    r"|  _|| |_| |  _ <| |_| | |___",
    r"|_|   \___/|_| \_\\____|_____|",
];
pub(super) fn banner() -> Vec<Line<'static>> {
    let mut lines = vec![Line::styled(
        "ILLUMITECH FORGE",
        Style::default().fg(TEAL).bold(),
    )];
    lines.extend(
        ASCII
            .iter()
            .map(|s| Line::styled((*s).to_owned(), Style::default().fg(ORANGE))),
    );
    lines
}
fn title(text: &str) -> Line<'static> {
    Line::styled(text.to_owned(), Style::default().fg(ORANGE).bold())
}
fn block(name: &str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(name.to_owned())
        .border_style(Style::default().fg(TEAL))
}
fn metric(value: &Value) -> String {
    value
        .as_u64()
        .map(|n| n.to_string())
        .unwrap_or_else(|| "não observado".into())
}
impl Chat {
    pub(super) fn draw(&mut self, f: &mut Frame, view: &View, root: &Path) {
        let area = f.area();
        if area.width < 40 || area.height < 16 {
            f.render_widget(
                Paragraph::new("FORGE · Amplie para 40 × 16. Ctrl+Q sai.")
                    .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        let session = view.sessions.get(&self.selected);
        self.transcript.update(session, self.activities);
        self.activity_panel.update(session);
        let approval = view
            .approvals
            .values()
            .find(|a| a["params"]["threadId"] == self.selected);
        self.choosing = approval.cloned();
        let active = view.connected && session.is_some_and(|s| s.active_turn.is_some());
        if active {
            self.active_started
                .get_or_insert_with(std::time::Instant::now);
        } else {
            self.active_started = None;
        }
        let matches = if self.panel.is_none() && !self.palette_dismissed {
            matching(&self.editor.text)
        } else {
            Vec::new()
        };
        self.palette_selection = self.palette_selection.min(matches.len().saturating_sub(1));
        let compact = area.height < 22;
        let palette_height = if matches.is_empty() {
            0
        } else {
            (matches.len().min(if compact { 3 } else { 5 }) + 2) as u16
        };
        let approval_height = if approval.is_some() { 6 } else { 0 };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(if compact { 2 } else { 3 }),
                Constraint::Min(1),
                Constraint::Length(approval_height),
                Constraint::Length(palette_height),
                Constraint::Length(if compact { 3 } else { 5 }),
                Constraint::Length(if compact { 2 } else { 4 }),
            ])
            .split(area);
        let model = session
            .and_then(|s| s.effective["model"].as_str())
            .unwrap_or("configuração nativa");
        f.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("ILLUMITECH FORGE", Style::default().fg(ORANGE).bold()),
                    Span::styled(
                        format!("  v{} · LOCAL", env!("CARGO_PKG_VERSION")),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
                Line::from(format!(
                    "{}  /  {}",
                    display(&root.file_name().unwrap_or_default().to_string_lossy()),
                    display(model)
                )),
            ])
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            chunks[0],
        );
        // Native approval scope always takes precedence over decorative panels.
        if let Some(a) = approval.filter(|_| self.panel.is_none()) {
            let mut lines = vec![
                title("Revise a solicitação nativa"),
                Line::from("PgUp/PgDn percorrem o escopo completo."),
            ];
            lines.extend(
                display(&serde_json::to_string_pretty(&a["params"]).unwrap_or_default())
                    .lines()
                    .map(|s| Line::from(s.to_owned())),
            );
            self.render_lines(f, lines, chunks[1]);
        } else if let Some(panel) = self.panel {
            self.render_panel(f, panel, view, root, chunks[1]);
        } else if self.transcript.lines.is_empty() {
            let mut lines = if chunks[1].height >= 12 && chunks[1].width >= 55 {
                banner()
            } else {
                vec![title("Bem-vindo ao Forge · /plan organiza uma tarefa")]
            };
            lines.extend([
                Line::default(),
                title("Uma ideia. Um plano. Evidências."),
                Line::from("Descreva o que você quer fazer neste projeto."),
                Line::default(),
                Line::from("/ comandos  ·  F1 ajuda  ·  Ctrl+R histórico"),
                Line::from("Conta e permissões gerenciadas pelo cliente nativo."),
            ]);
            f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), chunks[1]);
        } else {
            self.render_lines(f, self.transcript.lines.clone(), chunks[1]);
        }
        if let Some(a) = approval {
            let options = approval_choices(a)
                .iter()
                .enumerate()
                .map(|(i, c)| format!("Alt+{} {}", i + 1, decision_label(c)))
                .collect::<Vec<_>>()
                .join("  ·  ");
            let text = format!(
                "{}\n{}\nDecisão explícita; não aprova uma release Forge.",
                display(
                    a["params"]["reason"]
                        .as_str()
                        .unwrap_or("O cliente solicita sua decisão")
                ),
                if options.is_empty() {
                    "Solicitação não suportada: bloqueada."
                } else {
                    &options
                }
            );
            f.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(block("Permissão pendente")),
                chunks[2],
            );
        }
        if !matches.is_empty() {
            let rows = matches
                .iter()
                .map(|c| ListItem::new(format!("/{:<12} {}", c.name, c.description)))
                .collect::<Vec<_>>();
            let mut state = ListState::default().with_selected(Some(self.palette_selection));
            f.render_stateful_widget(
                List::new(rows)
                    .block(block("Comandos · ↑/↓ · Tab completa · Enter confirma"))
                    .highlight_symbol("› ")
                    .highlight_style(Style::default().fg(ORANGE).bold()),
                chunks[3],
                &mut state,
            );
        }
        let mut draft = self.editor.text.clone();
        draft.insert(self.editor.cursor, '▏');
        let name = if self.panel == Some(Panel::History) {
            "Busca ativa no painel · Esc volta ao rascunho"
        } else {
            "Mensagem · Enter envia · Ctrl+J nova linha"
        };
        let paragraph = Paragraph::new(draft)
            .wrap(Wrap { trim: false })
            .block(block(name));
        let caret_lines = Paragraph::new(self.editor.text[..self.editor.cursor].to_owned())
            .wrap(Wrap { trim: false })
            .line_count(chunks[4].width.saturating_sub(2));
        let offset = caret_lines
            .saturating_sub(chunks[4].height.saturating_sub(2) as usize)
            .min(u16::MAX as usize) as u16;
        f.render_widget(paragraph.scroll((offset, 0)), chunks[4]);
        let status = if !self.busy.is_empty() {
            busy_label(&self.busy).to_owned()
        } else if !view.connected {
            "Desconectado · /reconnect para reconectar".into()
        } else if view.account["authenticated"] != true {
            "Login necessário no cliente nativo.".into()
        } else if session.is_some_and(|s| !s.loaded) {
            "Sessão salva · /resume para recuperar o histórico.".into()
        } else if approval.is_some() {
            "Aguardando sua decisão".into()
        } else if active {
            let seconds = self
                .active_started
                .map(|s| s.elapsed().as_secs())
                .unwrap_or(0);
            format!(
                "{} Respondendo · {:02}:{:02} · Esc interrompe",
                ['◐', '◓', '◑', '◒'][self.tick % 4],
                seconds / 60,
                seconds % 60
            )
        } else {
            "Pronto · somente leitura · aprovação nativa sob demanda".into()
        };
        let notice = view.error.as_deref().unwrap_or(&self.notice);
        let usage = session
            .and_then(|s| s.usage["total"]["totalTokens"].as_u64())
            .map(|n| format!("{n} tokens observados"))
            .unwrap_or_else(|| "Tokens ainda não observados".into());
        let scroll = if self.scroll_back > 0 {
            " · PgDn volta ao fim"
        } else {
            ""
        };
        let footer = if compact {
            vec![
                Line::styled(status, Style::default().fg(TEAL)),
                Line::styled(
                    "F1 ajuda · F2 atividades · Ctrl+Q sai",
                    Style::default().fg(Color::Gray),
                ),
            ]
        } else {
            vec![
                Line::styled(status, Style::default().fg(TEAL)),
                Line::styled(display(notice), Style::default().fg(ORANGE)),
                Line::from(format!("{usage}{scroll}")),
                Line::styled(
                    "F1 ajuda · F2 atividades · Ctrl+R histórico · Ctrl+Q sai · Chat não comprova gates",
                    Style::default().fg(Color::Gray),
                ),
            ]
        };
        f.render_widget(Paragraph::new(footer), chunks[5]);
    }
    fn render_lines(&self, f: &mut Frame, lines: Vec<Line<'static>>, area: Rect) {
        let paragraph = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });
        let height = paragraph.line_count(area.width).min(u16::MAX as usize) as u16;
        let max = height.saturating_sub(area.height);
        f.render_widget(
            paragraph.scroll((max.saturating_sub(self.scroll_back), 0)),
            area,
        );
    }
    fn render_panel(&mut self, f: &mut Frame, panel: Panel, view: &View, root: &Path, area: Rect) {
        if panel == Panel::Activities {
            let lines = self.activity_panel.lines(
                area.width.saturating_sub(2),
                area.height.saturating_sub(2),
                &mut self.panel_scroll,
            );
            f.render_widget(
                Paragraph::new(lines)
                    .wrap(Wrap { trim: false })
                    .scroll((self.panel_scroll, 0))
                    .block(block(
                        "Atividades · chamadas, comandos e arquivos · Esc volta",
                    )),
                area,
            );
            return;
        }
        if panel == Panel::Sessions || panel == Panel::History {
            let rows = if panel == Panel::Sessions {
                view.sessions
                    .values()
                    .map(|s| {
                        format!(
                            "{}{}",
                            display(&s.title),
                            if s.id == self.selected {
                                " · atual"
                            } else {
                                ""
                            }
                        )
                    })
                    .collect::<Vec<_>>()
            } else {
                self.history_matches()
                    .iter()
                    .map(|s| s.replace('\n', " ↵ "))
                    .collect()
            };
            self.panel_selection = self.panel_selection.min(rows.len().saturating_sub(1));
            let name = if panel == Panel::Sessions {
                "Conversas deste projeto · Enter retoma · Esc volta".to_owned()
            } else {
                format!(
                    "Histórico da sessão · buscar: {} · Enter edita",
                    display(&self.history_query)
                )
            };
            let items = if rows.is_empty() {
                vec![ListItem::new("Nenhum resultado nesta sessão.")]
            } else {
                rows.into_iter().map(ListItem::new).collect()
            };
            let mut state = ListState::default().with_selected(Some(self.panel_selection));
            f.render_stateful_widget(
                List::new(items)
                    .block(block(&name))
                    .highlight_symbol("› ")
                    .highlight_style(Style::default().fg(ORANGE).bold()),
                area,
                &mut state,
            );
            return;
        }
        if panel == Panel::Workflow {
            let mut lines = vec![
                title("Plano, execução e evidências"),
                Line::from("PgUp/PgDn percorrem o escopo. Esc volta ao editor."),
                Line::from(
                    "/run DIGEST confirma o escopo; /tasks ID diagnostica; /tasks resume ID retoma.",
                ),
            ];
            lines.extend(
                display(&serde_json::to_string_pretty(&self.workflow).unwrap_or_default())
                    .lines()
                    .map(|s| Line::from(s.to_owned())),
            );
            self.render_lines(f, lines, area);
            return;
        }
        let lines = if panel == Panel::Help {
            let mut lines = vec![
                title("Ajuda do Forge"),
                Line::from("Esc fecha este painel. Nenhum comando é enviado ao modelo."),
                Line::default(),
                title("Conversa e edição"),
            ];
            for text in [
                "Enter envia · Ctrl+J / Alt+Enter insere nova linha",
                "↑/↓ recuperam pedidos quando o rascunho tem uma linha",
                "Ctrl+R busca no histórico desta sessão",
                "Ctrl+A/E início/fim da linha · Ctrl+W apaga palavra",
                "Ctrl+U/K recorta antes/depois do cursor · Ctrl+Y recupera",
                "Ctrl+C limpa com recuperação por Ctrl+Y; Esc interrompe turno",
                "PgUp/PgDn lê histórico · Ctrl+L redesenha a tela",
                "Ctrl+O expande atividades · Ctrl+Q sai",
                "F2 / /activities: ↑/↓ selecionam, Enter/Espaço alternam detalhes",
                "No painel: ←/→ recolhem/expandem cada chamada ou arquivo",
            ] {
                lines.push(Line::from(text));
            }
            lines.push(Line::default());
            lines.push(title("Comandos disponíveis"));
            lines.extend(
                COMMANDS
                    .iter()
                    .map(|c| Line::from(format!("/{:<12} {}", c.name, c.description))),
            );
            lines.push(Line::default());
            lines.push(Line::from(
                "Interromper não garante encerrar terminais nativos em segundo plano.",
            ));
            lines
        } else {
            let session = view.sessions.get(&self.selected);
            let total = session.map(|s| &s.usage["total"]);
            vec![
                title("Status da sessão"),
                Line::from(format!("Projeto: {}", display(&root.to_string_lossy()))),
                Line::from(format!(
                    "Conexão: {}",
                    if view.connected {
                        "conectado"
                    } else {
                        "desconectado"
                    }
                )),
                Line::from(format!(
                    "Conta nativa: {}",
                    if view.account["authenticated"] == true {
                        "autenticada"
                    } else {
                        "não verificada"
                    }
                )),
                Line::from(format!(
                    "Modelo: {}",
                    display(
                        session
                            .and_then(|s| s.effective["model"].as_str())
                            .unwrap_or("configuração nativa; ainda não observado")
                    )
                )),
                Line::from("Permissões: somente leitura; aprovação nativa sob demanda"),
                Line::from(format!(
                    "Sessão: {}",
                    session
                        .map(|s| display(&s.title))
                        .unwrap_or_else(|| "ainda não criada".into())
                )),
                Line::default(),
                title("Tokens observados"),
                Line::from(format!(
                    "Entrada: {}",
                    total
                        .map(|t| metric(&t["inputTokens"]))
                        .unwrap_or_else(|| "não observado".into())
                )),
                Line::from(format!(
                    "Cache incluído na entrada: {}",
                    total
                        .map(|t| metric(&t["cachedInputTokens"]))
                        .unwrap_or_else(|| "não observado".into())
                )),
                Line::from(format!(
                    "Saída: {}",
                    total
                        .map(|t| metric(&t["outputTokens"]))
                        .unwrap_or_else(|| "não observado".into())
                )),
                Line::from(format!(
                    "Total: {}",
                    total
                        .map(|t| metric(&t["totalTokens"]))
                        .unwrap_or_else(|| "não observado".into())
                )),
                Line::default(),
                Line::from("Ausência não significa zero; não é saldo da assinatura ou custo."),
                Line::from("Chat e permissões de ferramentas não aprovam gates Forge."),
                Line::from("Esc volta à conversa."),
            ]
        };
        // Informational panels start at the top; PageDown advances toward the end.
        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(
            paragraph
                .scroll((self.panel_scroll, 0))
                .block(block(if panel == Panel::Help {
                    "Ajuda · PgUp/PgDn"
                } else {
                    "Status · PgUp/PgDn"
                })),
            area,
        );
    }
}
