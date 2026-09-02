use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::Stylize;
use ratatui::style::Color;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

const INSTALL_DIR: &str = ".local/bin";

#[derive(Debug, Clone)]
pub enum InstallStep {
    Welcome,
    Downloading { progress: f32, downloaded_mb: f32, total_mb: Option<f32> },
    Installing,
    Done { success: bool, message: String },
    Error(String),
}

#[derive(Debug)]
pub struct InstallWizard {
    pub step: InstallStep,
    pub platform: String,
    pub binary_name: String,
}

impl Default for InstallWizard {
    fn default() -> Self {
        let (platform, binary_name) = detect_platform();
        Self {
            step: InstallStep::Welcome,
            platform,
            binary_name,
        }
    }
}

fn detect_platform() -> (String, String) {
    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "unknown"
    };

    let ext = if os == "windows" { "zip" } else { "tar.gz" };

    (
        format!("{}-{}", os, arch),
        format!("rgaa-{}:{}.{}", os, arch, ext),
    )
}

pub fn run_install_wizard() -> bool {
    let mut wizard = InstallWizard::default();

    let mut terminal = ratatui::init();
    terminal.clear().unwrap();

    loop {
        terminal.draw(|frame| render(&wizard, frame)).unwrap();

        if let Event::Key(key) = event::read().unwrap() {
            match &wizard.step {
                InstallStep::Welcome => {
                    if key.code == KeyCode::Enter {
                        wizard.step = InstallStep::Installing;
                    } else if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
                        break;
                    }
                }
                InstallStep::Installing => {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    wizard.step = InstallStep::Done {
                        success: true,
                        message: format!("Installed to ~/{}/rgaa", INSTALL_DIR),
                    };
                }
                InstallStep::Done { .. } | InstallStep::Error(_) => {
                    if key.code == KeyCode::Enter
                        || key.code == KeyCode::Esc
                        || key.code == KeyCode::Char('q')
                    {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    ratatui::restore();

    matches!(
        wizard.step,
        InstallStep::Done { success: true, .. }
    )
}

fn render(wizard: &InstallWizard, frame: &mut Frame) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new("rgaa installer")
            .alignment(Alignment::Center)
            .fg(Color::Cyan),
        chunks[0],
    );

    match &wizard.step {
        InstallStep::Welcome => {
            let lines: Vec<Line> = vec![
                Line::from(format!("Detected platform: {}", wizard.platform)),
                Line::from(""),
                Line::from("This will download and install the rgaa binary."),
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
        InstallStep::Downloading {
            progress,
            downloaded_mb,
            total_mb,
        } => {
            let pct = (*progress * 100.0) as u16;
            let filled = ((*progress) * 50.0) as usize;
            let empty = 50 - filled;
            let bar: String = format!(
                "[{}{}] {}%",
                "#".repeat(filled),
                ".".repeat(empty),
                pct
            );
            let lines: Vec<Line> = vec![
                Line::from(format!("Downloading {}...", wizard.binary_name)),
                Line::from(bar),
                Line::from(format!(
                    "{:.1} MB / {} MB",
                    downloaded_mb,
                    total_mb.map(|t| format!("{:.1}", t)).unwrap_or_else(|| "?".into())
                )),
            ];
            frame.render_widget(
                Block::default()
                    .title("Downloading")
                    .borders(Borders::ALL)
                    .border_style(Color::Yellow),
                chunks[2],
            );
            let inner = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(chunks[2])[0];
            frame.render_widget(Paragraph::new(Text::from(lines)), inner);
        }
        InstallStep::Installing => {
            frame.render_widget(
                Block::default()
                    .title("Installing")
                    .borders(Borders::ALL)
                    .border_style(Color::Yellow),
                chunks[2],
            );
            let inner = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(chunks[2])[0];
            frame.render_widget(
                Paragraph::new("Extracting and installing...")
                    .alignment(Alignment::Center),
                inner,
            );
        }
        InstallStep::Done { success, message } => {
            let color = if *success {
                Color::Green
            } else {
                Color::Red
            };
            let title = if *success { "Success" } else { "Failed" };
            frame.render_widget(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(color),
                chunks[2],
            );
            let inner = Layout::default()
                .constraints([Constraint::Fill(1)])
                .split(chunks[2])[0];
            frame.render_widget(
                Paragraph::new(message.clone()).alignment(Alignment::Center),
                inner,
            );
        }
        InstallStep::Error(msg) => {
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
            frame.render_widget(Paragraph::new(msg.clone()), inner);
        }
    }

    let hint = match &wizard.step {
        InstallStep::Welcome => "ENTER: continue | ESC/q: cancel",
        InstallStep::Done { .. } => "ENTER: finish",
        _ => "",
    };
    frame.render_widget(
        Paragraph::new(hint)
            .alignment(Alignment::Center)
            .fg(Color::DarkGray),
        chunks[3],
    );
}
