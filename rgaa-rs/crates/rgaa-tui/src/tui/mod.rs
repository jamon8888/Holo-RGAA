pub mod audit;
pub mod export;
pub mod history;
pub mod install;
pub mod setup;

pub use audit::run_audit_wizard;
pub use history::run_history_view;
pub use install::run_install_wizard;
pub use setup::run_setup_wizard;

use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::Stylize;
use ratatui::style::Color;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

enum MainMenuSelection {
    Audit,
    History,
    Settings,
    Exit,
}

/// Park the menu terminal while a subview takes over the screen, then bring
/// the menu back. Returns the revived terminal.
async fn suspend(
    terminal: ratatui::DefaultTerminal,
    task: impl core::future::Future<Output = ()>,
) -> ratatui::DefaultTerminal {
    drop(terminal);
    task.await;
    let mut terminal = ratatui::init();
    terminal.clear().unwrap();
    terminal
}

pub async fn run() {
    let mut terminal = ratatui::init();
    terminal.clear().unwrap();
    let mut selected = MainMenuSelection::Audit;

    loop {
        terminal
            .draw(|frame| render_main_menu(frame, &selected))
            .unwrap();

        if let Event::Key(key) = event::read().unwrap() {
            match key.code {
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        terminal = suspend(terminal, crate::tui::run_audit_wizard()).await;
                    }
                    KeyCode::Char('h') | KeyCode::Char('H') => {
                        terminal = suspend(terminal, crate::tui::run_history_view()).await;
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        terminal = suspend(
                            terminal,
                            async {
                                let _ = crate::tui::run_setup_wizard();
                            },
                        )
                        .await;
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        break;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        selected = match selected {
                            MainMenuSelection::Audit => MainMenuSelection::History,
                            MainMenuSelection::History => MainMenuSelection::Settings,
                            MainMenuSelection::Settings => MainMenuSelection::Exit,
                            MainMenuSelection::Exit => MainMenuSelection::Audit,
                        };
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        selected = match selected {
                            MainMenuSelection::Audit => MainMenuSelection::Exit,
                            MainMenuSelection::History => MainMenuSelection::Audit,
                            MainMenuSelection::Settings => MainMenuSelection::History,
                            MainMenuSelection::Exit => MainMenuSelection::Settings,
                        };
                    }
                    KeyCode::Enter => {
                        match selected {
                            MainMenuSelection::Audit => {
                                terminal =
                                    suspend(terminal, crate::tui::run_audit_wizard()).await;
                            }
                            MainMenuSelection::History => {
                                terminal =
                                    suspend(terminal, crate::tui::run_history_view()).await;
                            }
                            MainMenuSelection::Settings => {
                                terminal = suspend(
                                    terminal,
                                    async {
                                        let _ = crate::tui::run_setup_wizard();
                                    },
                                )
                                .await;
                            }
                            MainMenuSelection::Exit => {
                                break;
                            }
                        }
                    }
                    KeyCode::Esc => {
                        break;
                    }
                    _ => {}
                }
        }
    }

    ratatui::restore();
}

fn render_main_menu(frame: &mut Frame, selected: &MainMenuSelection) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new("rgaa — RGAA Accessibility Auditor")
            .alignment(Alignment::Center)
            .fg(Color::Cyan)
            .bold(),
        chunks[0],
    );

    let items = [
        (MainMenuSelection::Audit, "[A]udit URL", "Run a new accessibility audit"),
        (
            MainMenuSelection::History,
            "[H]istory",
            "View past audit results",
        ),
        (
            MainMenuSelection::Settings,
            "[S]ettings",
            "Configure API key and preferences",
        ),
        (MainMenuSelection::Exit, "[Q]uit", "Exit rgaa"),
    ];

    for (i, (sel, label, desc)) in items.iter().enumerate() {
        let is_selected = matches!(selected, s if std::mem::discriminant(s) == std::mem::discriminant(sel));
        let style = if is_selected {
            ratatui::style::Style::default()
                .fg(Color::Yellow)
                .bold()
        } else {
            ratatui::style::Style::default().fg(Color::White)
        };
        let text = if is_selected {
            format!("  {}  — {}", label, desc)
        } else {
            format!("    {}  — {}", label, desc)
        };
        frame.render_widget(Paragraph::new(text).style(style), chunks[i + 1]);
    }
}
