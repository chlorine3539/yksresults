use crate::data::{Exam, Tracker};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState};
use ratatui::{Terminal, widgets::Wrap};
use serde_json::Value;
use std::io;

#[derive(Clone, Copy)]
struct TopicRef {
    subject: usize,
    topic: usize,
}

struct App {
    tracker: Tracker,
    query: String,
    search_input: Option<String>,
    selected: usize,
    message: String,
    quit: bool,
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(io::stdout(), EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let result = run_loop(&mut terminal);
    let _ = terminal.show_cursor();
    drop(terminal);
    drop(guard);
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App {
        tracker: Tracker::load(Exam::Tyt)?,
        query: String::new(),
        search_input: None,
        selected: 0,
        message: String::new(),
        quit: false,
    };

    loop {
        let entries = filtered_topics(&app);
        if entries.is_empty() {
            app.selected = 0;
        } else {
            app.selected = app.selected.min(entries.len() - 1);
        }
        terminal.draw(|frame| draw(frame, &app, &entries))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        app.message.clear();
        if app.search_input.is_some() {
            handle_search(&mut app, key);
            continue;
        }
        handle_key(&mut app, key, &entries)?;
        if app.quit {
            return Ok(());
        }
    }
}

fn handle_key(
    app: &mut App,
    key: KeyEvent,
    entries: &[TopicRef],
) -> Result<(), Box<dyn std::error::Error>> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => app.quit = true,
        KeyCode::Up | KeyCode::Char('k') => app.selected = app.selected.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => {
            app.selected = (app.selected + 1).min(entries.len().saturating_sub(1));
        }
        KeyCode::PageUp => app.selected = app.selected.saturating_sub(10),
        KeyCode::PageDown => {
            app.selected = (app.selected + 10).min(entries.len().saturating_sub(1))
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            let exam = if app.tracker.exam == Exam::Tyt {
                Exam::Ayt
            } else {
                Exam::Tyt
            };
            app.tracker = Tracker::load(exam)?;
            app.selected = 0;
            app.message = format!("Loaded {}.", app.tracker.exam.label());
        }
        KeyCode::Char('/') => app.search_input = Some(app.query.clone()),
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.tracker = Tracker::load(app.tracker.exam)?;
            app.selected = 0;
            app.message = "Data reloaded.".into();
        }
        _ => {}
    }
    Ok(())
}

fn handle_search(app: &mut App, key: KeyEvent) {
    let Some(mut input) = app.search_input.take() else {
        return;
    };
    match key.code {
        KeyCode::Enter => {
            app.query = input.trim().to_owned();
            app.selected = 0;
        }
        KeyCode::Esc => {}
        KeyCode::Backspace => {
            input.pop();
            app.search_input = Some(input);
        }
        KeyCode::Char(character) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            input.push(character);
            app.search_input = Some(input);
        }
        _ => app.search_input = Some(input),
    }
}

fn filtered_topics(app: &App) -> Vec<TopicRef> {
    let query = app.query.to_lowercase();
    let mut entries = Vec::new();
    for (subject_index, subject) in app.tracker.topics["subjects"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let subject_name = subject.get("name").and_then(Value::as_str).unwrap_or("");
        for (topic_index, topic) in subject["topics"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            let topic_name = topic.get("name").and_then(Value::as_str).unwrap_or("");
            if query.is_empty()
                || subject_name.to_lowercase().contains(&query)
                || topic_name.to_lowercase().contains(&query)
            {
                entries.push(TopicRef {
                    subject: subject_index,
                    topic: topic_index,
                });
            }
        }
    }
    entries
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &App, entries: &[TopicRef]) {
    let area = frame.area();
    let stats = totals(&app.tracker.topics);
    let exam = app.tracker.exam.label();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(format!("yksresults | {exam} | {} topics", stats.total)),
        vertical[0],
    );
    let total_q = stats.correct + stats.wrong + stats.blank;
    let success_str = if total_q > 0 {
        format!("{:.1}%", (stats.correct as f64 / total_q as f64) * 100.0)
    } else {
        "—".to_string()
    };
    frame.render_widget(
        Paragraph::new(format!(
            "Questions: {}  Correct: {}  Wrong: {}  Blank: {}  Success: {}",
            total_q, stats.correct, stats.wrong, stats.blank, success_str
        )),
        vertical[1],
    );
    let search = app.search_input.as_deref().unwrap_or(&app.query);
    frame.render_widget(
        Paragraph::new(format!(
            "Search: {}",
            if search.is_empty() { "off" } else { search }
        )),
        vertical[2],
    );
    if !app.message.is_empty() {
        frame.render_widget(Paragraph::new(app.message.as_str()), vertical[3]);
    }

    let rows = entries
        .iter()
        .map(|location| {
            let subject = &app.tracker.topics["subjects"][location.subject];
            let topic = &subject["topics"][location.topic];
            let correct = topic["correct_total"].as_i64().unwrap_or(0);
            let wrong = topic["wrong_total"].as_i64().unwrap_or(0);
            let blank = topic["blank_total"].as_i64().unwrap_or(0);
            let total_q = correct + wrong + blank;
            let success_rate = if total_q > 0 {
                format!("{:.1}%", (correct as f64 / total_q as f64) * 100.0)
            } else {
                "—".to_string()
            };
            let last_result = match &topic["last_result"] {
                Value::Object(map) => {
                    let c = map.get("correct").and_then(Value::as_i64).unwrap_or(0);
                    let w = map.get("wrong").and_then(Value::as_i64).unwrap_or(0);
                    let b = map.get("blank").and_then(Value::as_i64).unwrap_or(0);
                    format!("{c}C {w}W {b}B")
                }
                Value::String(s) => s.clone(),
                _ => "—".to_string(),
            };
            Row::new(vec![
                Cell::from(subject["name"].as_str().unwrap_or("").to_owned()),
                Cell::from(topic["name"].as_str().unwrap_or("").to_owned()),
                Cell::from(total_q.to_string()),
                Cell::from(correct.to_string()),
                Cell::from(wrong.to_string()),
                Cell::from(blank.to_string()),
                Cell::from(success_rate),
                Cell::from(topic["last_tested"].as_str().unwrap_or("—").to_owned()),
                Cell::from(last_result),
            ])
        })
        .collect::<Vec<_>>();
    let widths = [
        Constraint::Length(16),
        Constraint::Min(22),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(9),
        Constraint::Length(12),
        Constraint::Length(10),
    ];
    let mut table_state = TableState::default();
    if !entries.is_empty() {
        table_state.select(Some(app.selected));
    }
    let table = Table::new(rows, widths)
        .header(Row::new([
            "SUBJECT",
            "TOPIC",
            "Q",
            "C",
            "W",
            "B",
            "SUCCESS",
            "LAST TESTED",
            "LAST RESULT",
        ]))
        .block(Block::default().borders(Borders::ALL))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(table, vertical[4], &mut table_state);

    let help = "↑/↓ move  / search  e exam  r reload  q quit";
    frame.render_widget(Paragraph::new(help), vertical[5]);
    if app.search_input.is_some() {
        let popup = centered_rect(60, 20, area);
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!("Search (Enter applies, Esc cancels):\n{}", search))
                .block(Block::default().title("Search").borders(Borders::ALL))
                .wrap(Wrap { trim: true }),
            popup,
        );
    }
}

struct Totals {
    total: usize,
    correct: i64,
    wrong: i64,
    blank: i64,
}

fn totals(data: &Value) -> Totals {
    let mut result = Totals {
        total: 0,
        correct: 0,
        wrong: 0,
        blank: 0,
    };
    for topic in data["subjects"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|subject| subject["topics"].as_array().into_iter().flatten())
    {
        result.total += 1;
        result.correct += topic["correct_total"].as_i64().unwrap_or(0);
        result.wrong += topic["wrong_total"].as_i64().unwrap_or(0);
        result.blank += topic["blank_total"].as_i64().unwrap_or(0);
    }
    result
}

fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    area: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
