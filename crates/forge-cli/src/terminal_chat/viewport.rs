//! A reading position anchored to a logical line, independent of appended output.
use super::*;
use ratatui::layout::Rect;

#[derive(Default)]
pub(super) struct Viewport {
    session: String,
    initialized: bool,
    pub following: bool,
    anchor: (usize, usize),
    rows: Vec<usize>,
    layout_revision: u64,
    width: u16,
    top: usize,
    maximum: usize,
    pub height: u16,
}
impl Viewport {
    pub fn reset(&mut self) {
        self.following = true;
        self.anchor = (0, 0);
    }
    pub fn latest(&mut self) {
        self.following = true;
    }
    pub fn start(&mut self) {
        self.following = false;
        self.set_top(0);
    }
    pub fn move_by(&mut self, delta: isize) {
        let top = self.top.saturating_add_signed(delta).min(self.maximum);
        self.following = top == self.maximum;
        self.set_top(top);
    }
    fn set_top(&mut self, top: usize) {
        self.top = top;
        let mut remaining = top;
        for (i, rows) in self.rows.iter().enumerate() {
            if remaining < *rows {
                self.anchor = (i, remaining);
                return;
            }
            remaining = remaining.saturating_sub(*rows);
        }
        self.anchor = (0, 0);
    }
    pub fn draw(
        &mut self,
        f: &mut Frame,
        lines: &[Line<'static>],
        area: Rect,
        session: &str,
        revision: u64,
    ) {
        if !self.initialized || self.session != session {
            self.session = session.into();
            self.initialized = true;
            self.width = 0;
            self.reset();
        }
        self.height = area.height;
        if self.layout_revision != revision
            || self.width != area.width
            || self.rows.len() != lines.len()
        {
            self.layout_revision = revision;
            self.width = area.width;
            self.rows = lines
                .iter()
                .map(|line| {
                    Paragraph::new(line.clone())
                        .wrap(Wrap { trim: false })
                        .line_count(area.width)
                        .max(1)
                })
                .collect();
        }
        let total: usize = self.rows.iter().sum();
        self.maximum = total.saturating_sub(area.height as usize);
        let anchored = self.rows.iter().take(self.anchor.0).sum::<usize>()
            + self.anchor.1.min(
                self.rows
                    .get(self.anchor.0)
                    .copied()
                    .unwrap_or(1)
                    .saturating_sub(1),
            );
        let top = if self.following {
            self.maximum
        } else {
            anchored.min(self.maximum)
        };
        self.set_top(top);
        let mut skip = top;
        let mut y = area.y;
        for (line, rows) in lines.iter().zip(&self.rows) {
            if skip >= *rows {
                skip -= rows;
                continue;
            }
            let height = (rows - skip).min(area.bottom().saturating_sub(y) as usize) as u16;
            if height == 0 {
                break;
            }
            f.render_widget(
                Paragraph::new(line.clone())
                    .wrap(Wrap { trim: false })
                    .scroll((skip.min(u16::MAX as usize) as u16, 0)),
                Rect::new(area.x, y, area.width, height),
            );
            y += height;
            skip = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    fn render(v: &mut Viewport, lines: &[Line<'static>], width: u16, height: u16) -> String {
        let mut t = Terminal::new(TestBackend::new(width, height)).unwrap();
        t.draw(|f| {
            v.draw(
                f,
                lines,
                f.area(),
                "session",
                v.layout_revision.wrapping_add(1),
            )
        })
        .unwrap();
        t.backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }
    #[test]
    fn reader_stays_on_the_same_lines_while_output_grows() {
        let mut v = Viewport::default();
        let mut lines: Vec<_> = (0..100)
            .map(|i| Line::from(format!("row-{i:03}")))
            .collect();
        assert!(render(&mut v, &lines, 30, 10).contains("row-099"));
        v.move_by(-30);
        let before = render(&mut v, &lines, 30, 10);
        lines.extend((100..150).map(|i| Line::from(format!("row-{i:03}"))));
        assert_eq!(render(&mut v, &lines, 30, 10), before);
        v.latest();
        assert!(render(&mut v, &lines, 30, 10).contains("row-149"));
    }
    #[test]
    fn resize_preserves_logical_anchor_and_long_history_is_reachable() {
        let mut v = Viewport::default();
        let lines: Vec<_> = (0..70_000)
            .map(|i| Line::from(format!("row-{i:05} {}", "word ".repeat(10))))
            .collect();
        assert!(render(&mut v, &lines, 80, 10).contains("row-69999"));
        v.start();
        v.move_by(30);
        render(&mut v, &lines, 80, 10);
        assert!(render(&mut v, &lines, 30, 10).starts_with("row-00030"));
        v.start();
        assert!(render(&mut v, &lines, 30, 10).starts_with("row-00000"));
    }
    #[test]
    fn session_switch_and_streaming_reflow_refresh_cached_measurements() {
        let mut v = Viewport::default();
        let mut t = Terminal::new(TestBackend::new(20, 5)).unwrap();
        let short = vec![Line::from("short")];
        t.draw(|f| v.draw(f, &short, f.area(), "one", 1)).unwrap();
        let long = vec![Line::from("word ".repeat(100))];
        t.draw(|f| v.draw(f, &long, f.area(), "one", 2)).unwrap();
        assert!(v.maximum > 0);
        v.start();
        t.draw(|f| v.draw(f, &short, f.area(), "two", 2)).unwrap();
        assert_eq!(v.maximum, 0);
        assert!(v.following);
    }
}
