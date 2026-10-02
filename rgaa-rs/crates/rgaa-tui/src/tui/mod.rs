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

/// Print a sub-view's error to stderr before the main menu takes the screen back.
///
/// These four call sites used to discard the result with `let _ =`. The error
/// was reachable only through `tracing::warn!`, which this binary installs a
/// subscriber for only under `--debug`, so a wizard that failed looked to the
/// user like a menu that had simply blinked. Printing is terminal-safe here:
/// the menu's terminal is dropped before the sub-view starts and the sub-view
/// restores its own before returning, so stderr is not fighting a raw-mode
/// screen.
fn report<T, E: std::fmt::Display>(result: Result<T, E>, what: &str) {
    if let Err(error) = result {
        eprintln!("{what} failed: {error}");
    }
}

pub async fn run() -> std::io::Result<()> {
    let mut terminal = ratatui::try_init()?;
    terminal.clear()?;
    let mut selected = MainMenuSelection::Audit;
    let mut show_menu = true;

    loop {
        if show_menu {
            terminal.draw(|frame| render_main_menu(frame, &selected))?;
        }

        if let Event::Key(key) = event::read()? {
            if show_menu {
                match key.code {
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        show_menu = false;
                        drop(terminal);
                        report(crate::tui::run_audit_wizard(), "audit wizard");
                        terminal = ratatui::try_init()?;
                        terminal.clear()?;
                    }
                    KeyCode::Char('h') | KeyCode::Char('H') => {
                        show_menu = false;
                        drop(terminal);
                        report(crate::tui::run_history_view().await, "history view");
                        terminal = ratatui::try_init()?;
                        terminal.clear()?;
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        show_menu = false;
                        drop(terminal);
                        report(crate::tui::run_setup_wizard(), "setup wizard");
                        terminal = ratatui::try_init()?;
                        terminal.clear()?;
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
                    KeyCode::Enter => match selected {
                        MainMenuSelection::Audit => {
                            show_menu = false;
                            drop(terminal);
                            report(crate::tui::run_audit_wizard(), "audit wizard");
                            terminal = ratatui::try_init()?;
                            terminal.clear()?;
                        }
                        MainMenuSelection::History => {
                            show_menu = false;
                            drop(terminal);
                            report(crate::tui::run_history_view().await, "history view");
                            terminal = ratatui::try_init()?;
                            terminal.clear()?;
                        }
                        MainMenuSelection::Settings => {
                            show_menu = false;
                            drop(terminal);
                            report(crate::tui::run_setup_wizard(), "setup wizard");
                            terminal = ratatui::try_init()?;
                            terminal.clear()?;
                        }
                        MainMenuSelection::Exit => {
                            break;
                        }
                    },
                    KeyCode::Esc => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    ratatui::restore();
    Ok(())
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
        (
            MainMenuSelection::Audit,
            "[A]udit URL",
            "Run a new accessibility audit",
        ),
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
        let is_selected =
            matches!(selected, s if std::mem::discriminant(s) == std::mem::discriminant(sel));
        let style = if is_selected {
            ratatui::style::Style::default().fg(Color::Yellow).bold()
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
