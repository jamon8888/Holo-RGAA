use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::Stylize;
use ratatui::style::Color;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

const HOLO3_BASE_URL_DEFAULT: &str = "https://api.holo3.ai/v1";

#[derive(Debug, Clone)]
pub enum SetupStep {
    Welcome,
    ApiKeyInput,
    ApiKeyConfirm { key: String },
    BaseUrlInput { api_key: String },
    Review { api_key: String, base_url: String },
    Done,
    Error(String),
}

#[derive(Debug)]
pub struct SetupWizard {
    pub step: SetupStep,
}

impl Default for SetupWizard {
    fn default() -> Self {
        Self {
            step: SetupStep::Welcome,
        }
    }
}

pub fn run_setup_wizard() -> bool {
    let mut wizard = SetupWizard::default();
    let mut input_buffer = String::new();

    let mut terminal = ratatui::init();
    terminal.clear().unwrap();

    loop {
        terminal
            .draw(|frame| render(&wizard, frame, &input_buffer))
            .unwrap();

        if let Event::Key(key) = event::read().unwrap() {
            match &wizard.step {
                SetupStep::Welcome => {
                    if key.code == KeyCode::Enter {
                        wizard.step = SetupStep::ApiKeyInput;
                    } else if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
                        break;
                    }
                }
                SetupStep::ApiKeyInput => {
                    match key.code {
                        KeyCode::Enter => {
                            if !input_buffer.is_empty() {
                                wizard.step = SetupStep::ApiKeyConfirm {
                                    key: input_buffer.clone(),
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
                            input_buffer.clear();
                            wizard.step = SetupStep::Welcome;
                        }
                        _ => {}
                    }
                }
                SetupStep::ApiKeyConfirm { key: api_key_str } => {
                    if key.code == KeyCode::Char('y') || key.code == KeyCode::Enter {
                        wizard.step = SetupStep::BaseUrlInput {
                            api_key: api_key_str.clone(),
                        };
                    } else if key.code == KeyCode::Char('n') || key.code == KeyCode::Esc {
                        wizard.step = SetupStep::ApiKeyInput;
                        input_buffer.clear();
                    }
                }
                SetupStep::BaseUrlInput { api_key: api_key_str } => {
                    match key.code {
                        KeyCode::Enter => {
                            let base_url = if input_buffer.is_empty() {
                                HOLO3_BASE_URL_DEFAULT.to_string()
                            } else {
                                std::mem::take(&mut input_buffer)
                            };
                            wizard.step = SetupStep::Review {
                                api_key: api_key_str.clone(),
                                base_url,
                            };
                        }
                        KeyCode::Char(c) => {
                            input_buffer.push(c);
                        }
                        KeyCode::Backspace => {
                            input_buffer.pop();
                        }
                        KeyCode::Esc => {
                            input_buffer.clear();
                            wizard.step = SetupStep::ApiKeyConfirm {
                                key: api_key_str.clone(),
                            };
                        }
                        _ => {}
                    }
                }
                SetupStep::Review { .. } => {
                    if key.code == KeyCode::Char('y') || key.code == KeyCode::Enter {
                        if let SetupStep::Review { api_key, .. } = &wizard.step {
                            if let Err(e) = crate::keyring::store_api_key(api_key) {
                                wizard.step = SetupStep::Error(e.to_string());
                            } else {
                                wizard.step = SetupStep::Done;
                            }
                        }
                    } else if key.code == KeyCode::Char('n') || key.code == KeyCode::Esc {
                        wizard.step = SetupStep::ApiKeyInput;
                        input_buffer.clear();
                    }
                }
                SetupStep::Done | SetupStep::Error(_) => {
                    break;
                }
            }
        }
    }

    ratatui::restore();

    matches!(wizard.step, SetupStep::Done)
}

fn masked_key(key: &str) -> String {
    if key.len() <= 8 {
        "*".repeat(key.len())
    } else {
        format!("{}...{}", &key[..4], "*".repeat(4))
    }
}

fn render(wizard: &SetupWizard, frame: &mut Frame, input: &str) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new("rgaa setup").alignment(Alignment::Center).fg(Color::Cyan),
        chunks[0],
    );

    match &wizard.step {
        SetupStep::Welcome => {
            let lines = vec![
                Line::from("Welcome to rgaa setup!"),
                Line::from(""),
                Line::from("This will configure your Holo3 API key."),
                Line::from(""),
                Line::from("Press ENTER to continue, ESC to cancel"),
            ];
            frame.render_widget(
                Block::default()
                    .title("Welcome")
                    .borders(Borders::ALL)
                    .border_style(Color::White),
                chunks[2],
            );
            let inner = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(chunks[2])[0];
            frame.render_widget(Paragraph::new(Text::from(lines)), inner);
        }
        SetupStep::ApiKeyInput => {
            let display = if input.is_empty() {
                "".to_string()
            } else {
                input.to_string()
            };
            let lines = vec![
                Line::from("Enter your Holo3 API key:"),
                Line::from(""),
                Line::from(format!("> {}", display)),
            ];
            frame.render_widget(
                Block::default()
                    .title("API Key")
                    .borders(Borders::ALL)
                    .border_style(Color::Yellow),
                chunks[2],
            );
            let inner = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(chunks[2])[0];
            frame.render_widget(Paragraph::new(Text::from(lines)), inner);
            frame.render_widget(
                Paragraph::new("ENTER: confirm | BS: delete | ESC: back")
                    .alignment(Alignment::Center)
                    .fg(Color::DarkGray),
                chunks[3],
            );
        }
        SetupStep::ApiKeyConfirm { key } => {
            let masked = masked_key(key);
            let lines = vec![
                Line::from(format!("API key: {}?", masked)),
                Line::from(""),
                Line::from("Store in OS keyring? [Y/n]"),
            ];
            frame.render_widget(
                Block::default()
                    .title("Confirm")
                    .borders(Borders::ALL)
                    .border_style(Color::White),
                chunks[2],
            );
            let inner = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(chunks[2])[0];
            frame.render_widget(Paragraph::new(Text::from(lines)), inner);
        }
        SetupStep::BaseUrlInput { api_key: _ } => {
            let display = if input.is_empty() {
                HOLO3_BASE_URL_DEFAULT.to_string()
            } else {
                input.to_string()
            };
            let lines = vec![
                Line::from("Holo3 Base URL:"),
                Line::from("(press ENTER for default)"),
                Line::from(format!("> {}", display)),
            ];
            frame.render_widget(
                Block::default()
                    .title("Base URL")
                    .borders(Borders::ALL)
                    .border_style(Color::White),
                chunks[2],
            );
            let inner = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(chunks[2])[0];
            frame.render_widget(Paragraph::new(Text::from(lines)), inner);
        }
        SetupStep::Review { api_key, base_url } => {
            let masked = masked_key(api_key);
            let lines = vec![
                Line::from("Configuration summary:"),
                Line::from(""),
                Line::from(format!("  API key:   {}", masked)),
                Line::from(format!("  Base URL:  {}", base_url)),
                Line::from(""),
                Line::from("Save? [Y/n]"),
            ];
            frame.render_widget(
                Block::default()
                    .title("Review")
                    .borders(Borders::ALL)
                    .border_style(Color::Green),
                chunks[2],
            );
            let inner = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(chunks[2])[0];
            frame.render_widget(Paragraph::new(Text::from(lines)), inner);
        }
        SetupStep::Done => {
            let lines = vec![
                Line::from("Setup complete!").fg(Color::Green),
                Line::from(""),
                Line::from("Your API key has been stored securely."),
            ];
            frame.render_widget(
                Block::default()
                    .title("Done")
                    .borders(Borders::ALL)
                    .border_style(Color::Green),
                chunks[2],
            );
            let inner = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(chunks[2])[0];
            frame.render_widget(Paragraph::new(Text::from(lines)), inner);
        }
        SetupStep::Error(msg) => {
            let lines = vec![
                Line::from("Failed to store API key:").fg(Color::Red),
                Line::from(""),
                Line::from(msg.as_str()),
            ];
            frame.render_widget(
                Block::default()
                    .title("Error")
                    .borders(Borders::ALL)
                    .border_style(Color::Red),
                chunks[2],
            );
            let inner = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(chunks[2])[0];
            frame.render_widget(Paragraph::new(Text::from(lines)), inner);
        }
    }

    let hint = match &wizard.step {
        SetupStep::Welcome => "ENTER: continue | ESC: cancel",
        SetupStep::Done | SetupStep::Error(_) => "ENTER: finish",
        _ => "",
    };
    frame.render_widget(
        Paragraph::new(hint)
            .alignment(Alignment::Center)
            .fg(Color::DarkGray),
        chunks[4],
    );
}
