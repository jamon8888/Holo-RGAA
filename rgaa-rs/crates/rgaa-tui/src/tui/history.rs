use crate::storage::{AuditSummary, Storage, StorageError};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::Stylize;
use ratatui::style::Color;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

/// How many audits the view loads at a time. Matches the default of
/// `rgaa history` closely enough that the two show the same recent work.
const PAGE_SIZE: usize = 50;

/// Status counts for one audit, read from the stored result on demand.
///
/// The list only needs the summary row; counting criteria means
/// deserializing the whole audit, so it happens when a row is selected,
/// not for every row up front.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CriterionCounts {
    pub passed: usize,
    pub failed: usize,
    pub needs_review: usize,
    pub errors: usize,
}

impl CriterionCounts {
    fn from_audit(audit: &rgaa_core::AuditResult) -> Self {
        let mut counts = Self::default();
        for result in audit.pages.iter().flat_map(|page| &page.criteria) {
            match result.status {
                rgaa_core::CriterionStatus::Pass => counts.passed += 1,
                rgaa_core::CriterionStatus::Fail => counts.failed += 1,
                rgaa_core::CriterionStatus::NeedsReview => counts.needs_review += 1,
                rgaa_core::CriterionStatus::Error => counts.errors += 1,
                rgaa_core::CriterionStatus::NotTested
                | rgaa_core::CriterionStatus::NotApplicable => {}
            }
        }
        counts
    }
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub audit_id: String,
    pub url: String,
    pub taux_global: f64,
    pub etat_conformite: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Filled the first time the entry is opened; `None` until then.
    pub counts: Option<CriterionCounts>,
}

impl HistoryEntry {
    fn from_summary(summary: &AuditSummary) -> Self {
        Self {
            audit_id: summary.id.clone(),
            url: summary.url.clone(),
            taux_global: summary.taux_global,
            etat_conformite: summary.etat_conformite.clone(),
            created_at: summary.created_at,
            counts: None,
        }
    }

    fn status_color(&self) -> Color {
        match self.etat_conformite.as_str() {
            "totale" => Color::Green,
            "partielle" => Color::Yellow,
            _ => Color::Red,
        }
    }

    /// One row of the list: short id, URL, score, date.
    fn row(&self) -> String {
        format!(
            "{} | {} | {:.1}% | {}",
            self.audit_id.chars().take(8).collect::<String>(),
            self.url.chars().take(40).collect::<String>(),
            self.taux_global,
            self.created_at.format("%Y-%m-%d %H:%M")
        )
    }
}

struct HistoryState {
    entries: Vec<HistoryEntry>,
    list_state: ListState,
    /// Set when a load or a detail read fails, shown instead of the detail.
    message: Option<String>,
}

impl HistoryState {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            list_state: ListState::default(),
            message: None,
        }
    }

    fn select_first(&mut self) {
        self.list_state
            .select((!self.entries.is_empty()).then_some(0));
    }

    /// Moving the selection also drops any message: it described the previous
    /// row's read, and leaving it up would attach it to the new one.
    fn next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.message = None;
        let i = match self.list_state.selected() {
            Some(i) if i >= self.entries.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// See [`Self::next`].
    fn previous(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.message = None;
        let i = match self.list_state.selected() {
            Some(0) | None => self.entries.len() - 1,
            Some(i) => i - 1,
        };
        self.list_state.select(Some(i));
    }

    fn reload(&mut self, storage: &Storage) {
        match load_entries(storage) {
            Ok(entries) => {
                self.entries = entries;
                self.message = None;
                self.select_first();
            }
            Err(e) => self.message = Some(format!("failed to read audit history: {e}")),
        }
    }

    /// Read the stored result for the selected row and cache its counts.
    ///
    /// Clears `message` first. The renderer draws it above everything else and
    /// only a successful `reload` used to clear it, so one unreadable row left
    /// its error on screen for every row selected afterwards — including rows
    /// whose details loaded perfectly well.
    fn load_detail(&mut self, storage: &Storage) {
        self.message = None;
        let Some(entry) = self
            .list_state
            .selected()
            .and_then(|i| self.entries.get_mut(i))
        else {
            return;
        };
        if entry.counts.is_some() {
            return;
        }
        match storage.get_audit(&entry.audit_id) {
            Ok(Some(audit)) => entry.counts = Some(CriterionCounts::from_audit(&audit)),
            Ok(None) => self.message = Some("this audit is no longer in the database".to_string()),
            Err(e) => self.message = Some(format!("failed to read audit: {e}")),
        }
    }
}

fn load_entries(storage: &Storage) -> Result<Vec<HistoryEntry>, StorageError> {
    Ok(storage
        .list_audits(PAGE_SIZE)?
        .iter()
        .map(HistoryEntry::from_summary)
        .collect())
}

/// Past audits, read from the same local database `rgaa history` prints from.
pub async fn run_history_view() -> Result<(), Box<dyn std::error::Error>> {
    // Open the database before taking over the screen: a failure here is a
    // plain error, not a half-initialised terminal.
    let storage = crate::storage::storage().await?;

    // `try_init` enables raw mode and enters the alternate screen before it
    // can still fail; on that path the restore below is never reached.
    let mut terminal = ratatui::try_init().inspect_err(|_| ratatui::restore())?;
    let result = history_loop(&mut terminal, &storage);
    ratatui::restore();
    result.map_err(Into::into)
}

/// Renders stored audits and loads a selected entry's criteria on demand.
/// Terminal I/O errors are returned; the caller restores.
fn history_loop(
    terminal: &mut ratatui::DefaultTerminal,
    storage: &Storage,
) -> Result<(), std::io::Error> {
    let mut state = HistoryState::new();
    state.reload(storage);
    terminal.clear()?;

    loop {
        terminal.draw(|frame| render_history_view(frame, &mut state))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => break,
                KeyCode::Down | KeyCode::Char('j') => state.next(),
                KeyCode::Up | KeyCode::Char('k') => state.previous(),
                KeyCode::Enter => state.load_detail(storage),
                KeyCode::Char('r') | KeyCode::Char('R') => state.reload(storage),
                _ => {}
            }
        }
    }

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
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Audit History (r: reload, q: back)");

    if state.entries.is_empty() {
        frame.render_widget(
            Paragraph::new("No audits yet. Run an audit from the main menu.")
                .block(block)
                .alignment(Alignment::Center)
                .fg(Color::DarkGray),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = state
        .entries
        .iter()
        .map(|entry| {
            ListItem::new(entry.row())
                .style(ratatui::style::Style::default().fg(entry.status_color()))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(ratatui::style::Style::default().fg(Color::Yellow).bold())
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut state.list_state);
}

fn render_history_detail(frame: &mut Frame, state: &mut HistoryState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Audit Details");

    if let Some(message) = &state.message {
        frame.render_widget(
            Paragraph::new(message.as_str())
                .block(block)
                .wrap(Wrap { trim: true })
                .fg(Color::Red),
            area,
        );
        return;
    }

    let selected = state
        .list_state
        .selected()
        .and_then(|i| state.entries.get(i));

    let Some(entry) = selected else {
        frame.render_widget(
            Paragraph::new("Select an audit to view details")
                .block(block)
                .alignment(Alignment::Center),
            area,
        );
        return;
    };

    let counts = match &entry.counts {
        Some(counts) => format!(
            "Passed: {}\nFailed: {}\nNeeds Review: {}\nErrors: {}",
            counts.passed, counts.failed, counts.needs_review, counts.errors
        ),
        None => "Press ENTER to load the criteria breakdown".to_string(),
    };

    let detail = format!(
        "Audit ID: {}\nURL: {}\nTaux Global: {:.1}%\nStatus: {}\nDate: {}\n\n{}",
        entry.audit_id,
        entry.url,
        entry.taux_global,
        entry.etat_conformite,
        entry.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
        counts
    );

    frame.render_widget(
        Paragraph::new(detail)
            .block(block)
            .wrap(Wrap { trim: true }),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn criterion(id: &str, status: rgaa_core::CriterionStatus) -> rgaa_core::CriterionResult {
        rgaa_core::CriterionResult {
            criterion_id: id.to_string(),
            title: "t".to_string(),
            classification: rgaa_core::Classification::Deterministe,
            status,
            violations: Vec::new(),
            confidence: None,
            justification: None,
            source: "test".to_string(),
            citations: Vec::new(),
            considered_sources: Vec::new(),
            tests: Vec::new(),
            automated_verdict: None,
            verdict_basis: Vec::new(),
            evidence: Vec::new(),
            confidence_calibration_version: None,
            review_required: false,
            review_reason: None,
            verified_status: None,
            review_events: Vec::new(),
        }
    }

    fn audit(statuses: &[rgaa_core::CriterionStatus]) -> rgaa_core::AuditResult {
        rgaa_core::AuditResult {
            audit_id: "a".to_string(),
            url: "https://example.com".to_string(),
            pages: vec![rgaa_core::PageResult {
                url: "https://example.com".to_string(),
                title: None,
                criteria: statuses
                    .iter()
                    .enumerate()
                    .map(|(i, s)| criterion(&format!("1.{i}"), s.clone()))
                    .collect(),
                compliance_rate: 0.0,
                crawl_depth: 0,
            }],
            total_criteria: statuses.len(),
            passed: 0,
            failed: 0,
            na: 0,
            overall_compliance: 0.0,
            taux_global: 0.0,
            coverage_percent: 0.0,
            etat_conformite: "non".to_string(),
            duration_ms: 0,
            audit_complete: false,
        }
    }

    fn summary(id: &str) -> AuditSummary {
        AuditSummary {
            id: id.to_string(),
            url: "https://example.com/a-fairly-long-path-that-gets-truncated".to_string(),
            taux_global: 61.4,
            etat_conformite: "partielle".to_string(),
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .expect("static timestamp parses")
                .with_timezone(&chrono::Utc),
        }
    }

    #[test]
    fn counts_every_status_bucket() {
        use rgaa_core::CriterionStatus::*;
        let counts = CriterionCounts::from_audit(&audit(&[
            Pass,
            Pass,
            Fail,
            NeedsReview,
            Error,
            NotTested,
            NotApplicable,
        ]));
        assert_eq!(
            counts,
            CriterionCounts {
                passed: 2,
                failed: 1,
                needs_review: 1,
                errors: 1,
            }
        );
    }

    #[test]
    fn entry_mirrors_the_stored_summary() {
        let entry = HistoryEntry::from_summary(&summary("0123456789abcdef"));
        assert_eq!(entry.audit_id, "0123456789abcdef");
        assert_eq!(entry.taux_global, 61.4);
        assert_eq!(entry.status_color(), Color::Yellow);
        // Counts are not known until the full audit is read.
        assert!(entry.counts.is_none());
    }

    /// The id is cut to 8 characters and the url to 40, so a long audit id
    /// and a long path still leave the rate and the date visible.
    ///
    /// Asserted whole with `assert_eq!` rather than as a `starts_with`: the
    /// first version of this test used a hand-counted prefix, got the count
    /// wrong, and then failed with nothing but the expected string — the
    /// actual row was never printed, so the mismatch took a CI round trip to
    /// diagnose. Comparing the whole string makes the failure say what it got.
    #[test]
    fn row_shortens_id_and_url() {
        let row = HistoryEntry::from_summary(&summary("0123456789abcdef")).row();
        assert_eq!(
            row,
            "01234567 | https://example.com/a-fairly-long-path-t | 61.4% | 2026-01-02 03:04"
        );
    }

    #[test]
    fn selection_wraps_in_both_directions() {
        let mut state = HistoryState::new();
        state.entries = vec![
            HistoryEntry::from_summary(&summary("a")),
            HistoryEntry::from_summary(&summary("b")),
        ];
        state.select_first();
        assert_eq!(state.list_state.selected(), Some(0));
        state.previous();
        assert_eq!(state.list_state.selected(), Some(1));
        state.next();
        assert_eq!(state.list_state.selected(), Some(0));
    }

    #[test]
    fn empty_history_has_no_selection() {
        let mut state = HistoryState::new();
        state.select_first();
        state.next();
        state.previous();
        assert_eq!(state.list_state.selected(), None);
    }
}
