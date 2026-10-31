use super::{KeyCode, KeyEvent, KeyModifiers, display};

#[derive(Default)]
pub(super) struct Editor {
    pub text: String,
    pub cursor: usize,
    pub saved: String,
    history_index: Option<usize>,
    history_draft: String,
}
impl Editor {
    pub fn set(&mut self, text: String) {
        self.text = text;
        self.cursor = self.text.len();
    }
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.history_index = None;
    }
    pub fn insert(&mut self, text: &str) {
        let text = display(text);
        if self.text.len() + text.len() > 64 * 1024 {
            return;
        }
        self.text.insert_str(self.cursor, &text);
        self.cursor += text.len();
    }
    fn previous(&self) -> usize {
        self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i)
    }
    fn next(&self) -> usize {
        self.cursor
            + self.text[self.cursor..]
                .chars()
                .next()
                .map_or(0, char::len_utf8)
    }
    fn line_start(&self) -> usize {
        self.text[..self.cursor].rfind('\n').map_or(0, |i| i + 1)
    }
    fn line_end(&self) -> usize {
        self.cursor
            + self.text[self.cursor..]
                .find('\n')
                .unwrap_or(self.text.len() - self.cursor)
    }
    fn kill(&mut self, start: usize, end: usize) {
        if start == end {
            return;
        }
        self.saved = self.text.drain(start..end).collect();
        self.cursor = start;
    }
    pub fn clear_recoverable(&mut self) {
        self.saved = self.text.clone();
        self.clear();
    }
    pub fn history(&mut self, entries: &[String], older: bool) {
        if entries.is_empty() {
            return;
        }
        if older {
            if self.history_index.is_none() {
                self.history_draft = self.text.clone();
            }
            let index = self
                .history_index
                .map_or(entries.len() - 1, |i| i.saturating_sub(1));
            self.history_index = Some(index);
            self.set(entries[index].clone());
        } else if let Some(index) = self.history_index {
            if index + 1 < entries.len() {
                self.history_index = Some(index + 1);
                self.set(entries[index + 1].clone());
            } else {
                self.history_index = None;
                self.set(self.history_draft.clone());
            }
        }
    }
    pub fn edit(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl {
            match key.code {
                KeyCode::Char('a') => self.cursor = self.line_start(),
                KeyCode::Char('e') => self.cursor = self.line_end(),
                KeyCode::Char('u') => self.kill(self.line_start(), self.cursor),
                KeyCode::Char('k') => self.kill(self.cursor, self.line_end()),
                KeyCode::Char('w') => {
                    let mut start = self.cursor;
                    while start > 0
                        && self.text[..start]
                            .chars()
                            .next_back()
                            .unwrap()
                            .is_whitespace()
                    {
                        start -= self.text[..start].chars().next_back().unwrap().len_utf8();
                    }
                    while start > 0
                        && !self.text[..start]
                            .chars()
                            .next_back()
                            .unwrap()
                            .is_whitespace()
                    {
                        start -= self.text[..start].chars().next_back().unwrap().len_utf8();
                    }
                    self.kill(start, self.cursor);
                }
                KeyCode::Char('y') => self.insert(&self.saved.clone()),
                KeyCode::Char('j') => self.insert("\n"),
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::ALT) => {
                self.insert(&c.to_string())
            }
            KeyCode::Backspace => self.kill_without_yank(self.previous(), self.cursor),
            KeyCode::Delete => self.kill_without_yank(self.cursor, self.next()),
            KeyCode::Left => self.cursor = self.previous(),
            KeyCode::Right => self.cursor = self.next(),
            KeyCode::Home => self.cursor = self.line_start(),
            KeyCode::End => self.cursor = self.line_end(),
            KeyCode::Up | KeyCode::Down => {
                let column = self.text[self.line_start()..self.cursor].chars().count();
                let start = if key.code == KeyCode::Up {
                    let end = self.line_start().saturating_sub(1);
                    self.text[..end].rfind('\n').map_or(0, |i| i + 1)
                } else {
                    let end = self.line_end();
                    if end == self.text.len() {
                        return;
                    }
                    end + 1
                };
                let line = &self.text[start..];
                let count = line
                    .split('\n')
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(column)
                    .map(char::len_utf8)
                    .sum::<usize>();
                self.cursor = start + count;
            }
            _ => {}
        }
    }
    fn kill_without_yank(&mut self, start: usize, end: usize) {
        self.text.drain(start..end);
        self.cursor = start;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }
    #[test]
    fn readline_unicode_kill_yank_and_multiline_keep_boundaries() {
        let mut editor = Editor::default();
        editor.insert("ação 🚀");
        editor.edit(key('w'));
        assert_eq!(editor.text, "ação ");
        editor.edit(key('y'));
        assert_eq!(editor.text, "ação 🚀");
        editor.edit(key('j'));
        editor.insert("segunda linha");
        editor.edit(key('a'));
        editor.edit(key('k'));
        assert_eq!(editor.text, "ação 🚀\n");
        editor.edit(key('y'));
        assert_eq!(editor.text, "ação 🚀\nsegunda linha");
        editor.clear_recoverable();
        editor.edit(key('y'));
        assert_eq!(editor.text, "ação 🚀\nsegunda linha");
    }
    #[test]
    fn history_restores_unsent_draft_and_does_not_persist() {
        let mut editor = Editor::default();
        editor.insert("rascunho");
        let history = vec!["um".into(), "dois".into()];
        editor.history(&history, true);
        assert_eq!(editor.text, "dois");
        editor.history(&history, true);
        assert_eq!(editor.text, "um");
        editor.history(&history, false);
        editor.history(&history, false);
        assert_eq!(editor.text, "rascunho");
    }
}
