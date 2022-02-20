use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
enum State {
    Todo,
    InProgress,
    Blocked,
    Cancelled,
    Done,
}

#[derive(Serialize, Deserialize, Debug)]
enum LogEntryType {
    Opened,
    Comment(String),
    StateChangedTo(State),
}

#[derive(Serialize, Deserialize, Debug)]
struct LogEntry {
    #[serde(with = "chrono::serde::ts_seconds")]
    timestamp: chrono::DateTime<chrono::Utc>,
    entry_type: LogEntryType,
}

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    title: String,
    description: String,
    id: usize,
    log: Vec<LogEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Project {
    name: String,
    tasks: Vec<Task>,
}

struct AppState {}

impl AppState {
    fn initialize() -> AppState {
        AppState {}
    }
}

fn main() {
    if let Err(e) = actual_main() {
        eprintln!("ERROR: {}", e);
    }
    crossterm::terminal::disable_raw_mode().expect("Couldn't disable raw mode");
}

fn actual_main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = AppState::initialize();

    crossterm::terminal::enable_raw_mode().expect("Couldn't put terminal in raw mode");

    let (event_tx, event_rx) = std::sync::mpsc::channel();

    // Spawn event reader thread
    std::thread::spawn(move || loop {
        event_tx
            .send(crossterm::event::read().expect("Failed to read event"))
            .unwrap();
    });

    let mut terminal = tui::Terminal::new(tui::backend::CrosstermBackend::new(std::io::stdout()))?;
    terminal.clear()?;

    loop {
        terminal.draw(|rect| {
            draw_ui(rect, &app);
        });
        match event_rx.recv().unwrap() {
            crossterm::event::Event::Key(event) => match event.code {
                crossterm::event::KeyCode::Esc => break,
                _ => {}
            },
            _ => {}
        }
    }

    terminal.clear()?;
    Ok(())
}

fn draw_ui<B>(rect: &mut tui::Frame<B>, app: &AppState)
where
    B: tui::backend::Backend,
{
    let size = rect.size();
    // Vertical layout
    let chunks = tui::layout::Layout::default()
        .direction(tui::layout::Direction::Horizontal)
        .constraints(
            [
                tui::layout::Constraint::Percentage(20),
                tui::layout::Constraint::Percentage(80),
            ]
            .as_ref(),
        )
        .split(size);

    let left_sidebar = tui::widgets::Block::default()
        .title("Projects")
        .borders(tui::widgets::Borders::ALL)
        .border_style(tui::style::Style::default().fg(tui::style::Color::White))
        .style(tui::style::Style::default().bg(tui::style::Color::Black));

    rect.render_widget(left_sidebar, chunks[0]);
}
