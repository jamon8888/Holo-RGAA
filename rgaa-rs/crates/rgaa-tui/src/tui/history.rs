use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::Stylize;
use ratatui::style::Color;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;
use std::time::Duration;

/// Interactive history view over past audits.
pub struct HistoryView {
    /// Audits newest-first, as returned by `storage::list_audits`.
    pub audits: Vec<crate::storage::AuditSummary>,
    pub selected: usize,
    /// Load failure to display instead of the empty state. `None` on success.
    pub load_error: Option<String>,
}

impl HistoryView {
    pub fn new(audits: Vec<crate::storage::AuditSummary>) -> Self {
        Self {
            audits,
            selected: 0,
            load_error: None,
        }
    }

    /// Move the selection, clamped to the list bounds. No-op when empty.
    pub fn move_selection(&mut self, delta: i32) {
        if self.audits.is_empty() {
            self.selected = 0;
            return;
        }
        let next = self.selected as i32 + delta;
        self.selected = next.clamp(0, self.audits.len() as i32 - 1) as usize;
    }

    /// [url (max 38 chars), score, date, short id] per audit, newest first.
    /// Same columns as the CLI `rgaa history` (URL, Score, Date) plus the
    /// short id the CLI actually prints in its third column.
    pub fn rows(&self) -> Vec<[String; 4]> {
        self.audits
            .iter()
            .map(|a| {
                let short_id = a.id.split('-').next().unwrap_or(&a.id).to_string();
                [
                    a.url.chars().take(38).collect(),
                    format!("{:.1}%", a.taux_global),
                    a.created_at.format("%Y-%m-%d").to_string(),
                    short_id,
                ]
            })
            .collect()
    }
}

/// Show past audits. Storage failures render as an error, never as an
/// empty list, so a broken database can't masquerade as "no audits".
pub async fn run_history_view() {
    let (audits, load_error) = match crate::storage::storage().await {
        Ok(store) => match store.list_audits(50) {
            Ok(audits) => (audits, None),
            Err(e) => (Vec::new(), Some(format!("failed to list audits: {e}"))),
        },
        Err(e) => (Vec::new(), Some(format!("failed to open database: {e}"))),
    };
    let mut view = HistoryView {
        audits,
        selected: 0,
        load_error,
    };
    let mut terminal = ratatui::init();
    terminal.clear().unwrap();

    loop {
        terminal.draw(|frame| render_history(&view, frame)).unwrap();
        if event::poll(Duration::from_millis(200)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Down | KeyCode::Char('j') => view.move_selection(1),
                    KeyCode::Up | KeyCode::Char('k') => view.move_selection(-1),
                    _ => {}
                }
            }
        }
    }

    ratatui::restore();
}

fn render_history(view: &HistoryView, frame: &mut Frame) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new("audit history")
            .alignment(Alignment::Center)
            .fg(Color::Cyan),
        chunks[0],
    );

    if let Some(ref err) = view.load_error {
        let lines = vec![
            Line::from("History unavailable:").fg(Color::Red),
            Line::from(""),
            Line::from(err.as_str()),
        ];
        frame.render_widget(
            Paragraph::new(Text::from(lines)).alignment(Alignment::Center),
            chunks[1],
        );
    } else if view.audits.is_empty() {
        let lines = vec![
            Line::from("No audits found."),
            Line::from("Run `rgaa audit <URL>` first."),
        ];
        frame.render_widget(
            Paragraph::new(Text::from(lines)).alignment(Alignment::Center),
            chunks[1],
        );
    } else {
        let rows: Vec<Row> = view
            .rows()
            .into_iter()
            .enumerate()
            .map(|(i, [url, score, date, id])| {
                let row = Row::new(vec![url, score, date, id]);
                if i == view.selected {
                    row.style(ratatui::style::Style::default().fg(Color::Yellow).bold())
                } else {
                    row
                }
            })
            .collect();
        let widths = [
            Constraint::Length(40),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Fill(1),
        ];
        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["URL", "Score", "Date", "ID"])
                    .style(ratatui::style::Style::default().fg(Color::White).bold()),
            )
            .block(Block::default().borders(Borders::ALL).title("Audits"));
        frame.render_widget(table, chunks[1]);
    }

    frame.render_widget(
        Paragraph::new("↑/↓: select   q/ESC: back")
            .alignment(Alignment::Center)
            .fg(Color::DarkGray),
        chunks[2],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn summary(id: &str, url: &str) -> crate::storage::AuditSummary {
        crate::storage::AuditSummary {
            id: id.to_string(),
            url: url.to_string(),
            taux_global: 83.3,
            etat_conformite: "Conforme".to_string(),
            created_at: chrono::Utc.with_ymd_and_hms(2026, 9, 17, 12, 0, 0).unwrap(),
        }
    }

    #[test]
    fn rows_truncate_long_urls_and_shorten_ids() {
        let view = HistoryView::new(vec![summary(
            "abc123-def456",
            "https://example.com/a-very-long-path-that-exceeds-limits",
        )]);
        let rows = view.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], "https://example.com/a-very-long-path-t");
        assert_eq!(rows[0][1], "83.3%");
        assert_eq!(rows[0][2], "2026-09-17");
        assert_eq!(rows[0][3], "abc123");
    }

    #[test]
    fn move_selection_clamps_at_bounds() {
        let mut view = HistoryView::new(vec![
            summary("id-1", "https://a.test"),
            summary("id-2", "https://b.test"),
        ]);
        assert_eq!(view.selected, 0);
        view.move_selection(-1);
        assert_eq!(view.selected, 0);
        view.move_selection(1);
        assert_eq!(view.selected, 1);
        view.move_selection(5);
        assert_eq!(view.selected, 1);
        view.move_selection(-5);
        assert_eq!(view.selected, 0);
    }

    #[test]
    fn empty_history_selects_nothing() {
        let mut view = HistoryView::new(vec![]);
        assert!(view.rows().is_empty());
        view.move_selection(1);
        assert_eq!(view.selected, 0);
    }
}
