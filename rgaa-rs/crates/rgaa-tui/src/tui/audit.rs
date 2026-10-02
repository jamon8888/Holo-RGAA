use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::Stylize;
use ratatui::style::Color;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum AuditStep {
    UrlInput,
    Running {
        phase: String,
        progress: f32,
    },
    ResultsSummary {
        audit: rgaa_core::AuditResult,
    },
    DrillDown {
        audit: rgaa_core::AuditResult,
        criterion_id: String,
    },
    Error(String),
}

#[derive(Debug)]
pub struct AuditWizard {
    pub step: AuditStep,
    pub url: String,
    pub table_state: TableState,
    pub pending: Option<PendingAudit>,
}

#[derive(Debug)]
pub struct PendingAudit {
    pub rx: mpsc::Receiver<rgaa_orchestrator::AuditEvent>,
    pub phases: Vec<rgaa_orchestrator::AuditPhase>,
    pub done: Option<Result<rgaa_core::AuditResult, String>>,
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

/// Tick between redraws while the wizard waits for input. Short enough that
/// audit progress reads as live, long enough not to spin the CPU.
const TICK: Duration = Duration::from_millis(100);

pub fn run_audit_wizard() -> std::io::Result<()> {
    let mut terminal = ratatui::try_init()?;
    let result = audit_loop(&mut terminal);
    // Restore either way: a terminal left in raw mode is worse than the error
    // that caused it.
    ratatui::restore();
    result
}

fn audit_loop(terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
    let mut wizard = AuditWizard::default();
    let mut input_buffer = String::new();
    terminal.clear()?;

    loop {
        terminal.draw(|frame| render_audit(&mut wizard, frame, &input_buffer))?;

        // Poll instead of blocking on `event::read`: the audit runs on its own
        // thread, so the progress screen has to repaint — and reach
        // `ResultsSummary` — without the user touching the keyboard.
        if event::poll(TICK)? {
            if let Event::Key(key) = event::read()? {
                // Windows reports press *and* release; only act on the press.
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match &wizard.step {
                    AuditStep::UrlInput => match key.code {
                        KeyCode::Enter => {
                            if !input_buffer.is_empty() {
                                wizard.url = input_buffer.clone();
                                input_buffer.clear();

                                let (tx, rx) = mpsc::channel();
                                let url = wizard.url.clone();
                                let tx_done = tx.clone();
                                thread::spawn(move || run_audit_thread(url, tx, tx_done));

                                wizard.pending = Some(PendingAudit {
                                    rx,
                                    phases: Vec::new(),
                                    done: None,
                                });
                                wizard.step = AuditStep::Running {
                                    phase: "Starting audit...".to_string(),
                                    progress: 0.0,
                                };
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
                    },
                    AuditStep::Running { .. } => {
                        if key.code == KeyCode::Char('q') {
                            break;
                        }
                    }
                    AuditStep::ResultsSummary { .. } => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            break;
                        }
                        KeyCode::Down => {
                            let max = visible_criteria_count(&wizard.step);
                            let new_idx = (wizard.table_state.selected().unwrap_or(0) + 1)
                                .min(max.saturating_sub(1));
                            wizard.table_state.select(Some(new_idx));
                        }
                        KeyCode::Up => {
                            let new_idx =
                                wizard.table_state.selected().unwrap_or(0).saturating_sub(1);
                            wizard.table_state.select(Some(new_idx));
                        }
                        KeyCode::Enter => {
                            // Drill into the row the table actually shows — an
                            // audited criterion, not the n-th catalog entry.
                            if let AuditStep::ResultsSummary { audit } = &wizard.step {
                                let selected = wizard
                                    .table_state
                                    .selected()
                                    .and_then(|idx| {
                                        audit.pages.first().and_then(|p| p.criteria.get(idx))
                                    })
                                    .map(|r| r.criterion_id.clone());
                                if let Some(criterion_id) = selected {
                                    wizard.step = AuditStep::DrillDown {
                                        audit: audit.clone(),
                                        criterion_id,
                                    };
                                }
                            }
                        }
                        _ => {}
                    },
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

        // Drain events from the audit task
        if let Some(pending) = wizard.pending.as_mut() {
            loop {
                match pending.rx.try_recv() {
                    Ok(rgaa_orchestrator::AuditEvent::Phase(phase)) => {
                        pending.phases.push(phase);
                        wizard.step = AuditStep::Running {
                            phase: phase.label().to_string(),
                            progress: phase.progress(),
                        };
                    }
                    Ok(rgaa_orchestrator::AuditEvent::Done(result)) => {
                        pending.done = Some(result);
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        pending.done = Some(Err(
                            "audit task ended without returning a result".to_string()
                        ));
                        break;
                    }
                }
            }

            if let Some(done) = pending.done.take() {
                match done {
                    Ok(audit) => {
                        wizard.table_state.select(Some(0));
                        wizard.step = AuditStep::ResultsSummary { audit };
                    }
                    Err(e) => {
                        wizard.step = AuditStep::Error(e);
                    }
                }
                wizard.pending = None;
            }
        }
    }

    Ok(())
}

/// Number of rows the results table renders, which bounds the selection.
fn visible_criteria_count(step: &AuditStep) -> usize {
    match step {
        AuditStep::ResultsSummary { audit } | AuditStep::DrillDown { audit, .. } => audit
            .pages
            .first()
            .map(|p| p.criteria.len())
            .unwrap_or_default(),
        _ => 0,
    }
}

/// Body of the worker thread that runs one audit.
///
/// The wizard owns the terminal on the calling thread, so the audit gets a
/// runtime of its own and reports back over `tx`. The work itself goes
/// through the same [`rgaa_orchestrator::Orchestrator`] the headless
/// `rgaa audit` path uses — the TUI adds no second pipeline.
fn run_audit_thread(
    url: String,
    tx: mpsc::Sender<rgaa_orchestrator::AuditEvent>,
    tx_done: mpsc::Sender<rgaa_orchestrator::AuditEvent>,
) {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            let _ = tx_done.send(rgaa_orchestrator::AuditEvent::Done(Err(format!(
                "failed to start the audit runtime: {e}"
            ))));
            return;
        }
    };

    rt.block_on(async move {
        let orchestrator = rgaa_orchestrator::Orchestrator::new();
        let config = rgaa_core::CrawlConfig::default();
        let result = orchestrator
            .run_with_progress(&url, &config, move |phase| {
                let _ = tx.send(rgaa_orchestrator::AuditEvent::Phase(phase));
            })
            .await;

        if let Ok(audit) = &result {
            crate::storage::record_audit(audit).await;
        }

        let _ = tx_done.send(rgaa_orchestrator::AuditEvent::Done(result));
    });
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

fn render_audit(wizard: &mut AuditWizard, frame: &mut Frame, input: &str) {
    // Split the borrow up front: the results table needs the step to read
    // from and the table state to render into, at the same time.
    let AuditWizard {
        step, table_state, ..
    } = wizard;
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
        Paragraph::new("rgaa audit")
            .alignment(Alignment::Center)
            .fg(Color::Cyan),
        chunks[0],
    );

    match &*step {
        AuditStep::UrlInput => {
            let display = if input.is_empty() {
                "https://example.com".to_string()
            } else {
                input.to_string()
            };
            let lines = vec![
                Line::from("Enter target URL:"),
                Line::from(format!("> {}", display)),
            ];
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
                            .title("Criteria (↑/↓: select, ENTER: detail, q: back)")
                            .style(ratatui::style::Style::default()),
                    )
                    .highlight_style(ratatui::style::Style::default().fg(Color::Yellow).bold())
                    .highlight_symbol("▶ ");
                // Stateful so the row the user is about to drill into is the
                // row they can see highlighted.
                frame.render_stateful_widget(table, chunks[1], table_state);
            }
        }
        AuditStep::DrillDown {
            audit,
            criterion_id,
        } => {
            let criterion = rgaa_core::RgaaCriteria::all()
                .iter()
                .find(|c| c.id == *criterion_id)
                .cloned();

            let result = audit
                .pages
                .first()
                .and_then(|p| p.criteria.iter().find(|r| r.criterion_id == *criterion_id));

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
                    lines.push(Line::from(format!("{} violation(s):", r.violations.len())));
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
