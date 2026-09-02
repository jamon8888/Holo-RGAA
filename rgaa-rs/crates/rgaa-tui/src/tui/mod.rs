pub mod install;
pub mod setup;

pub use install::run_install_wizard;
pub use setup::run_setup_wizard;

use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::layout::Alignment;

pub async fn run() {
    let mut terminal = ratatui::init();
    terminal.clear().unwrap();
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let text = ratatui::text::Text::from("rgaa TUI — press Ctrl+C to exit");
            let block = ratatui::widgets::Paragraph::new(text)
                .block(
                    ratatui::widgets::Block::default()
                        .title("rgaa")
                        .borders(ratatui::widgets::Borders::ALL),
                )
                .style(ratatui::style::Style::default().fg(ratatui::style::Color::Green))
                .alignment(Alignment::Center);
            frame.render_widget(block, area);
        }).unwrap();

        if let Event::Key(key) = event::read().unwrap() {
            if key.code == KeyCode::Char('q')
                || (key.code == KeyCode::Char('c')
                    && key.modifiers.contains(ratatui::crossterm::event::KeyModifiers::CONTROL))
            {
                break;
            }
        }
    }
    ratatui::restore();
}
