use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::Stylize;
use ratatui::style::Color;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table, TableState};
use ratatui::Frame;

#[derive(Debug, Clone)]
pub enum AuditStep {
    UrlInput,
    Running { phase: String, progress: f32 },
    ResultsSummary { audit: rgaa_core::AuditResult },
    DrillDown { audit: rgaa_core::AuditResult, criterion_id: String },
    Error(String),
}

#[derive(Debug)]
pub struct AuditWizard {
    pub step: AuditStep,
    pub url: String,
    pub table_state: TableState,
}

impl Default for AuditWizard {
    fn default() -> Self {
        Self {
            step: AuditStep::UrlInput,
            url: String::new(),
            table_state: TableState::default(),
        }
    }
}

pub fn run_audit_wizard() {
    let mut wizard = AuditWizard::default();
    let mut input_buffer = String::new();
    let mut terminal = ratatui::init();
    terminal.clear().unwrap();

    loop {
        terminal
            .draw(|frame| render_audit(&wizard, frame, &input_buffer))
            .unwrap();

        if let Event::Key(key) = event::read().unwrap() {
            match &wizard.step {
                AuditStep::UrlInput => {
                    match key.code {
                        KeyCode::Enter => {
                            if !input_buffer.is_empty() {
                                wizard.url = input_buffer.clone();
                                wizard.step = AuditStep::Running {
                                    phase: "Starting audit...".to_string(),
                                    progress: 0.0,
                                };
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
                                    if let AuditStep::ResultsSummary { audit, .. } = &wizard.step {
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
