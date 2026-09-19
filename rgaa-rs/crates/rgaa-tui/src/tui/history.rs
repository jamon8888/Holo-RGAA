use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::Stylize;
use ratatui::style::Color;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, ScrollbarState, Wrap};
use ratatui::Frame;
use rgaa_storage::{AuditSummary, PostgresStorage, Storage};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub audit_id: String,
    pub url: String,
    pub taux_global: f64,
    pub etat_conformite: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub passed: usize,
    pub failed: usize,
    pub needs_review: usize,
    pub errors: usize,
}

impl HistoryEntry {
    fn from_summary(summary: &AuditSummary) -> Self {
        Self {
            audit_id: summary.id.clone(),
            url: summary.url.clone(),
            taux_global: summary.taux_global,
            etat_conformite: summary.etat_conformite.clone(),
            created_at: summary.created_at,
            passed: 0,
            failed: 0,
            needs_review: 0,
            errors: 0,
        }
    }

    fn status_color(&self) -> Color {
        match self.etat_conformite.as_str() {
            "totale" => Color::Green,
            "partielle" => Color::Yellow,
            _ => Color::Red,
        }
    }
}

struct HistoryState {
    entries: Vec<HistoryEntry>,
    list_state: ListState,
    detail_scroll: usize,
    detail_scroll_state: ScrollbarState,
    loading: bool,
}

impl HistoryState {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            list_state: ListState::default(),
            detail_scroll: 0,
            detail_scroll_state: ScrollbarState::default(),
            loading: false,
        }
    }

    fn next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.entries.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.entries.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn scroll_detail_down(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_add(1);
        self.detail_scroll_state = self.detail_scroll_state.position(self.detail_scroll);
    }

    fn scroll_detail_up(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_sub(1);
        self.detail_scroll_state = self.detail_scroll_state.position(self.detail_scroll);
    }
}

pub async fn run_history_view() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();
    terminal.clear()?;

    let mut state = HistoryState::new();
    state.loading = true;

    // Load history
    let storage = PostgresStorage::new("postgres://localhost/rgaa").await?;
    if let Ok(summaries) = storage.list_audits(50, 0).await {
        for summary in summaries {
            state.entries.push(HistoryEntry::from_summary(&summary));
        }
    }
    state.loading = false;

    let storage = Arc::new(Mutex::new(storage));

    loop {
        terminal.draw(|frame| render_history_view(frame, &mut state))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => break,
                KeyCode::Down | KeyCode::Char('j') => state.next(),
                KeyCode::Up | KeyCode::Char('k') => state.previous(),
                KeyCode::PageDown => state.scroll_detail_down(),
                KeyCode::PageUp => state.scroll_detail_up(),
                KeyCode::Enter => {
                    if let Some(idx) = state.list_state.selected() {
                        if let Some(entry) = state.entries.get(idx) {
                            show_detail(entry).await;
                        }
                    }
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    // Reload
                    state.loading = true;
                    let storage_guard = storage.lock().await;
                    if let Ok(summaries) = storage_guard.list_audits(50, 0).await {
                        state.entries.clear();
                        for summary in summaries {
                            state.entries.push(HistoryEntry::from_summary(&summary));
                        }
                    }
                    state.loading = false;
                }
                _ => {}
            }
        }
    }

    ratatui::restore();
    Ok(())
}

fn render_history_view(frame: &mut Frame, state: &mut HistoryState) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_history_list(frame, state, chunks[0]);
    render_history_detail(frame, state, chunks[1]);
}

fn render_history_list(frame: &mut Frame, state: &mut HistoryState, area: Rect) {
    let items: Vec<ListItem> = state
        .entries
        .iter()
        .map(|entry| {
            let status = format!("{:.1}%", entry.taux_global);
            let status_color = entry.status_color();
            let date = entry.created_at.format("%Y-%m-%d %H:%M").to_string();
            let content = format!(
                "{} | {} | {} | {}",
                entry.audit_id[..8.min(entry.audit_id.len())].to_string(),
                entry.url.chars().take(40).collect::<String>(),
                status,
                date
            );
            ListItem::new(content).style(ratatui::style::Style::default().fg(status_color))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Audit History"))
        .highlight_style(ratatui::style::Style::default().fg(Color::Yellow).bold())
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut state.list_state);
}

fn render_history_detail(frame: &mut Frame, state: &mut HistoryState, area: Rect) {
    let selected = state.list_state.selected().and_then(|i| state.entries.get(i));

    let (content, _scroll_state) = if let Some(entry) = selected {
        let detail = format!(
            "Audit ID: {}\nURL: {}\nTaux Global: {:.1}%\nStatus: {}\nDate: {}\nPassed: {}\nFailed: {}\nNeeds Review: {}\nErrors: {}",
            entry.audit_id,
            entry.url,
            entry.taux_global,
            entry.etat_conformite,
            entry.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
            entry.passed,
            entry.failed,
            entry.needs_review,
            entry.errors
        );
        let paragraph = Paragraph::new(detail)
            .block(Block::default().borders(Borders::ALL).title("Audit Details"))
            .wrap(Wrap { trim: true })
            .scroll((state.detail_scroll as u16, 0));
        (paragraph, state.detail_scroll_state.clone())
    } else {
        let paragraph = Paragraph::new("Select an audit to view details")
            .block(Block::default().borders(Borders::ALL).title("Audit Details"))
            .alignment(Alignment::Center);
        (paragraph, state.detail_scroll_state.clone())
    };

    frame.render_widget(content, area);
}

async fn show_detail(_entry: &HistoryEntry) {
    // In a real implementation, this would show a modal or push a detail view
    // For now, just a placeholder
}
