use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::Stylize;
use ratatui::style::Color;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;
use std::time::Duration;

pub struct HistoryView {
    pub audits: Vec<crate::storage::AuditSummary>,
    pub selected: usize,
}

impl HistoryView {
    pub fn new(audits: Vec<crate::storage::AuditSummary>) -> Self {
        Self { audits, selected: 0 }
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.audits.is_empty() {
            self.selected = 0;
            return;
        }
        let next = self.selected as i32 + delta;
        self.selected = next.clamp(0, self.audits.len() as i32 - 1) as usize;
    }

    /// [url (max 38 chars), score, short id] per audit, newest first.
    /// Matches the CLI `rgaa history` column layout.
    pub fn rows(&self) -> Vec<[String; 3]> {
        self.audits
            .iter()
            .map(|a| {
                let short_id = a.id.split('-').next().unwrap_or(&a.id).to_string();
                [
                    a.url.chars().take(38).collect(),
                    format!("{:.1}%", a.taux_global),
                    short_id,
                ]
            })
            .collect()
    }
}

pub async fn run_history_view() {
    let audits = match crate::storage::storage().await {
        Ok(store) => store.list_audits(50).unwrap_or_default(),
        Err(e) => {
            tracing::warn!("tui: failed to open storage: {e}");
            Vec::new()
        }
    };
    let mut view = HistoryView::new(audits);
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
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new("audit history")
            .alignment(Alignment::Center)
            .fg(Color::Cyan),
        chunks[0],
    );

    if view.audits.is_empty() {
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
            .map(|(i, [url, score, id])| {
                let row = Row::new(vec![url, score, id]);
                if i == view.selected {
                    row.style(
                        ratatui::style::Style::default()
                            .fg(Color::Yellow)
                            .bold(),
                    )
                } else {
                    row
                }
            })
            .collect();
        let widths = [
            Constraint::Length(40),
            Constraint::Length(8),
            Constraint::Fill(1),
        ];
        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["URL", "Score", "ID"])
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
        assert_eq!(rows[0][2], "abc123");
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
