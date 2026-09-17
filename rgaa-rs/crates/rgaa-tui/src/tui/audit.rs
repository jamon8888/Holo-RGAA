use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::Stylize;
use ratatui::style::Color;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Debug, Clone)]
pub enum AuditStep {
    UrlInput,
    Running { phase: String, progress: f32 },
    ResultsSummary { audit: rgaa_core::AuditResult },
    DrillDown { audit: rgaa_core::AuditResult, criterion_id: String },
    Error(String),
}

pub struct AuditWizard {
    pub step: AuditStep,
    pub url: String,
    pub table_state: TableState,
    pending: Option<UnboundedReceiver<Result<rgaa_core::AuditResult, String>>>,
}

impl Default for AuditWizard {
    fn default() -> Self {
        Self {
            step: AuditStep::UrlInput,
            url: String::new(),
            table_state: TableState::default(),
            pending: None,
        }
    }
}

impl AuditWizard {
    /// Transition UrlInput → Running for a non-empty URL. Returns false (no
    /// transition) when the input is empty.
    pub fn submit_url(&mut self, input: &str) -> bool {
        if input.is_empty() {
            return false;
        }
        self.url = input.to_string();
        self.step = AuditStep::Running {
            phase: "Starting audit...".to_string(),
            progress: 0.0,
        };
        true
    }

    /// Land on ResultsSummary or Error once the spawned audit finishes.
    pub fn apply_audit_result(&mut self, result: Result<rgaa_core::AuditResult, String>) {
        match result {
            Ok(audit) => {
                self.step = AuditStep::ResultsSummary { audit };
            }
            Err(err) => {
                self.step = AuditStep::Error(err);
            }
        }
    }

    /// Advance the indeterminate progress indicator; no-op unless Running.
    /// The orchestrator exposes no progress callback, so this is a spinner,
    /// not a measurement.
    pub fn tick(&mut self) {
        if let AuditStep::Running { progress, .. } = &mut self.step {
            *progress = (*progress + 0.05) % 1.0;
        }
    }
}

fn spawn_audit(url: String) -> UnboundedReceiver<Result<rgaa_core::AuditResult, String>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        let config = rgaa_core::CrawlConfig {
            max_pages: 10,
            max_depth: 3,
            respect_robots: true,
            sample_mode: false,
        };
        let result = rgaa_orchestrator::pipeline::Orchestrator::new()
            .run(&url, &config)
            .await;
        let _ = tx.send(result);
    });
    rx
}

/// Best-effort persistence so finished audits show up in History.
/// A storage failure must never hide real audit results.
async fn persist_audit_best_effort(audit: &rgaa_core::AuditResult) {
    match crate::storage::storage().await {
        Ok(store) => {
            if let Err(e) = store.save_audit(audit) {
                tracing::warn!("tui: failed to persist audit: {e}");
            }
        }
        Err(e) => tracing::warn!("tui: failed to open storage: {e}"),
    }
}

pub async fn run_audit_wizard() {
    let mut wizard = AuditWizard::default();
    let mut input_buffer = String::new();
    let mut terminal = ratatui::init();
    terminal.clear().unwrap();

    loop {
        terminal
            .draw(|frame| render_audit(&wizard, frame, &input_buffer))
            .unwrap();

        if event::poll(Duration::from_millis(120)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                match &wizard.step {
                    AuditStep::UrlInput => {
                        match key.code {
                            KeyCode::Enter => {
                                if wizard.submit_url(&input_buffer) {
                                    wizard.pending = Some(spawn_audit(wizard.url.clone()));
                                    input_buffer.clear();
                                }
                            }
                            KeyCode::Char(c) => {
                                input_buffer.push(c);
                            }
                            KeyCode::Backspace => {
                                input_buffer.pop();
                            }
                            KeyCode::Esc => {
                                break;
                            }
                            _ => {}
                        }
                    }
                AuditStep::Running { .. } => {
                    if key.code == KeyCode::Char('q') {
                        break;
                    }
                }
                AuditStep::ResultsSummary { .. } => {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            break;
                        }
                        KeyCode::Down => {
                            let max = rgaa_core::RgaaCriteria::all().len();
                            let new_idx = (wizard.table_state.selected().unwrap_or(0) + 1)
                                .min(max.saturating_sub(1));
                            wizard.table_state.select(Some(new_idx));
                        }
                        KeyCode::Up => {
                            let new_idx = wizard
                                .table_state
                                .selected()
                                .unwrap_or(0)
                                .saturating_sub(1);
                            wizard.table_state.select(Some(new_idx));
                        }
                        KeyCode::Enter => {
                            if let Some(idx) = wizard.table_state.selected() {
                                let criteria = rgaa_core::RgaaCriteria::all();
                                if idx < criteria.len() {
                                    let criterion = &criteria[idx];
                                    if let AuditStep::ResultsSummary { audit, .. } = &wizard.step
                                    {
                                        wizard.step = AuditStep::DrillDown {
                                            audit: audit.clone(),
                                            criterion_id: criterion.id.to_string(),
                                        };
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                AuditStep::DrillDown { .. } => {
                    if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
                        if let AuditStep::DrillDown { audit, .. } = &wizard.step {
                            wizard.step = AuditStep::ResultsSummary {
                                audit: audit.clone(),
                            };
                        }
                    }
                }
                AuditStep::Error(_) => {
                    if key.code == KeyCode::Enter || key.code == KeyCode::Esc {
                        break;
                    }
                }
                }
            }
        }

        // While an audit is in flight: spin the progress indicator and land
        // on ResultsSummary (or Error) as soon as the spawned task delivers.
        if matches!(wizard.step, AuditStep::Running { .. }) {
            wizard.tick();
            if let Some(rx) = wizard.pending.as_mut() {
                if let Ok(result) = rx.try_recv() {
                    wizard.pending = None;
                    match result {
                        Ok(audit) => {
                            persist_audit_best_effort(&audit).await;
                            wizard.apply_audit_result(Ok(audit));
                        }
                        Err(err) => wizard.apply_audit_result(Err(err)),
                    }
                }
            }
        }
    }

    ratatui::restore();
}

fn status_color(status: &rgaa_core::CriterionStatus) -> Color {
    match status {
        rgaa_core::CriterionStatus::Pass => Color::Green,
        rgaa_core::CriterionStatus::Fail => Color::Red,
        rgaa_core::CriterionStatus::NotTested => Color::DarkGray,
        rgaa_core::CriterionStatus::NeedsReview => Color::Yellow,
        rgaa_core::CriterionStatus::NotApplicable => Color::Blue,
        rgaa_core::CriterionStatus::Error => Color::Red,
    }
}

fn status_label(status: &rgaa_core::CriterionStatus) -> &'static str {
    match status {
        rgaa_core::CriterionStatus::Pass => "PASS",
        rgaa_core::CriterionStatus::Fail => "FAIL",
        rgaa_core::CriterionStatus::NotTested => "N/A",
        rgaa_core::CriterionStatus::NeedsReview => "REVIEW",
        rgaa_core::CriterionStatus::NotApplicable => "N/A",
        rgaa_core::CriterionStatus::Error => "ERROR",
    }
}

fn render_audit(wizard: &AuditWizard, frame: &mut Frame, input: &str) {
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
        Paragraph::new("rgaa audit").alignment(Alignment::Center).fg(Color::Cyan),
        chunks[0],
    );

    match &wizard.step {
        AuditStep::UrlInput => {
            let display = if input.is_empty() {
                "https://example.com".to_string()
            } else {
                input.to_string()
            };
            let lines =
                vec![Line::from("Enter target URL:"), Line::from(format!("> {}", display))];
            frame.render_widget(
                Block::default()
                    .title("URL")
                    .borders(Borders::ALL)
                    .border_style(Color::Yellow),
                chunks[1],
            );
            let inner = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(chunks[1])[0];
            frame.render_widget(Paragraph::new(Text::from(lines)), inner);
        }
        AuditStep::Running { phase, progress } => {
            let pct = (*progress * 100.0) as u16;
            frame.render_widget(
                Block::default()
                    .title("Running Audit")
                    .borders(Borders::ALL)
                    .border_style(Color::Yellow),
                chunks[1],
            );
            let inner = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(chunks[1])[0];
            frame.render_widget(
                Paragraph::new(format!("{}\n{:.0}%", phase, pct)).alignment(Alignment::Center),
                inner,
            );
        }
        AuditStep::ResultsSummary { audit } => {
            let taux = audit.taux_global;
            let label = if taux >= 80.0 {
                "Conforme"
            } else if taux >= 50.0 {
                "Partiellement conforme"
            } else {
                "Non conforme"
            };
            let color = if taux >= 80.0 {
                Color::Green
            } else if taux >= 50.0 {
                Color::Yellow
            } else {
                Color::Red
            };
            let lines = vec![
                Line::from(format!("URL: {}", audit.url)),
                Line::from(format!("Score: {:.1}% ({})", taux, label)),
                Line::from(format!(
                    "Passed: {} | Failed: {} | N/A: {}",
                    audit.passed, audit.failed, audit.na
                )),
            ];
            frame.render_widget(Paragraph::new(Text::from(lines)).fg(color), chunks[0]);

            if let Some(page) = audit.pages.first() {
                let criteria = rgaa_core::RgaaCriteria::all();
                let rows: Vec<Row> = page
                    .criteria
                    .iter()
                    .map(|result| {
                        let criterion = criteria
                            .iter()
                            .find(|c| c.id == result.criterion_id)
                            .map(|c| c.id.to_string())
                            .unwrap_or_else(|| result.criterion_id.clone());
                        Row::new(vec![
                            criterion,
                            status_label(&result.status).to_string(),
                            result.title.clone(),
                        ])
                    })
                    .collect();

                let widths = [
                    Constraint::Length(6),
                    Constraint::Length(8),
                    Constraint::Fill(1),
                ];
                let table = Table::new(rows, widths)
                    .header(
                        Row::new(vec!["ID", "Status", "Topic"])
                            .style(ratatui::style::Style::default().fg(Color::White).bold()),
                    )
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Criteria")
                            .style(ratatui::style::Style::default()),
                    );
                frame.render_widget(table, chunks[1]);
            }
        }
        AuditStep::DrillDown { audit, criterion_id } => {
            let criterion = rgaa_core::RgaaCriteria::all()
                .iter()
                .find(|c| c.id == *criterion_id)
                .cloned();

            let result = audit.pages.first().and_then(|p| {
                p.criteria
                    .iter()
                    .find(|r| r.criterion_id == *criterion_id)
            });

            let mut lines: Vec<Line> = vec![];

            if let Some(c) = &criterion {
                lines.push(Line::from(format!("Criterion {} — {}", c.id, c.title)));
                lines.push(Line::from(""));
            }
            if let Some(r) = result {
                lines.push(
                    Line::from(format!("Status: {}", status_label(&r.status)))
                        .fg(status_color(&r.status)),
                );
                if let Some(ref just) = r.justification {
                    lines.push(Line::from(format!("Justification: {}", just)));
                }
                if let Some(conf) = r.confidence {
                    lines.push(Line::from(format!("Confidence: {:.0}%", conf * 100.0)));
                }
                if !r.violations.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(format!(
                        "{} violation(s):",
                        r.violations.len()
                    )));
                    for v in &r.violations {
                        lines.push(Line::from(format!(
                            "  - [{}] {} ({} node(s))",
                            v.impact, v.description, v.nodes_affected
                        )));
                    }
                }
            }

            frame.render_widget(
                Block::default()
                    .title("Criterion Detail")
                    .borders(Borders::ALL)
                    .border_style(Color::White),
                chunks[1],
            );
            let inner = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(chunks[1])[0];
            frame.render_widget(Paragraph::new(Text::from(lines)).scroll((0, 0)), inner);
            frame.render_widget(
                Paragraph::new("ESC: back")
                    .alignment(Alignment::Center)
                    .fg(Color::DarkGray),
                chunks[2],
            );
        }
        AuditStep::Error(msg) => {
            let lines = vec![
                Line::from("Audit failed:").fg(Color::Red),
                Line::from(""),
                Line::from(msg.as_str()),
            ];
            frame.render_widget(
                Block::default()
                    .title("Error")
                    .borders(Borders::ALL)
                    .border_style(Color::Red),
                chunks[1],
            );
            let inner = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(chunks[1])[0];
            frame.render_widget(Paragraph::new(Text::from(lines)), inner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_audit() -> rgaa_core::AuditResult {
        rgaa_core::AuditResult {
            audit_id: "test-id".into(),
            url: "https://example.com".into(),
            pages: vec![],
            total_criteria: 106,
            passed: 10,
            failed: 2,
            na: 0,
            overall_compliance: 83.3,
            taux_global: 83.3,
            coverage_percent: 100.0,
            etat_conformite: "Conforme".into(),
            duration_ms: 100,
        }
    }

    #[test]
    fn submit_empty_url_stays_on_input() {
        let mut wizard = AuditWizard::default();
        assert!(!wizard.submit_url(""));
        assert!(matches!(wizard.step, AuditStep::UrlInput));
    }

    #[test]
    fn submit_url_enters_running() {
        let mut wizard = AuditWizard::default();
        assert!(wizard.submit_url("https://example.com"));
        assert_eq!(wizard.url, "https://example.com");
        assert!(matches!(wizard.step, AuditStep::Running { .. }));
    }

    #[test]
    fn apply_ok_lands_on_results_summary() {
        let mut wizard = AuditWizard::default();
        wizard.submit_url("https://example.com");
        wizard.apply_audit_result(Ok(sample_audit()));
        match &wizard.step {
            AuditStep::ResultsSummary { audit } => assert_eq!(audit.url, "https://example.com"),
            other => panic!("expected ResultsSummary, got {:?}", other),
        }
    }

    #[test]
    fn apply_err_lands_on_error() {
        let mut wizard = AuditWizard::default();
        wizard.submit_url("https://example.com");
        wizard.apply_audit_result(Err("boom".to_string()));
        assert!(matches!(wizard.step, AuditStep::Error(_)));
    }

    #[test]
    fn tick_advances_progress_only_while_running() {
        let mut wizard = AuditWizard::default();
        wizard.tick();
        assert!(matches!(wizard.step, AuditStep::UrlInput));
        wizard.submit_url("https://example.com");
        let before = match wizard.step {
            AuditStep::Running { progress, .. } => progress,
            _ => unreachable!(),
        };
        wizard.tick();
        let after = match wizard.step {
            AuditStep::Running { progress, .. } => progress,
            _ => unreachable!(),
        };
        assert!(after > before, "tick must advance progress while running");
    }
}
