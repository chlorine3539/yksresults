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

#[derive(Clone, Copy, Eq, PartialEq)]
enum Language {
    English,
    Turkish,
}

impl Language {
    fn from_system() -> Self {
        if is_turkish_locale(sys_locale::get_locale().as_deref()) {
            Self::Turkish
        } else {
            Self::English
        }
    }

    fn name(self, item: &Value) -> &str {
        let keys: &[&str] = match self {
            Self::English => &["name_en", "name"],
            Self::Turkish => &["name", "name_en"],
        };
        keys.iter()
            .find_map(|key| item.get(*key).and_then(Value::as_str))
            .unwrap_or("")
    }

    fn title(self, exam: &str, total: usize) -> String {
        match self {
            Self::English => format!("yksresults | {exam} | {total} topics"),
            Self::Turkish => format!("yksresults | {exam} | {total} konu"),
        }
    }

    fn summary(self, total_q: i64, correct: i64, wrong: i64, blank: i64, success: &str) -> String {
        match self {
            Self::English => format!(
                "Questions: {total_q}  Correct: {correct}  Wrong: {wrong}  Blank: {blank}  Success: {success}"
            ),
            Self::Turkish => format!(
                "Sorular: {total_q}  Doğru: {correct}  Yanlış: {wrong}  Boş: {blank}  Başarı: {success}"
            ),
        }
    }

    fn search(self, search: &str) -> String {
        let (label, off) = match self {
            Self::English => ("Search", "off"),
            Self::Turkish => ("Arama", "kapalı"),
        };
        format!("{label}: {}", if search.is_empty() { off } else { search })
    }

    fn loaded(self, exam: &str) -> String {
        match self {
            Self::English => format!("Loaded {exam}."),
            Self::Turkish => format!("{exam} yüklendi."),
        }
    }

    fn reload_message(self) -> &'static str {
        match self {
            Self::English => "Data reloaded.",
            Self::Turkish => "Veriler yeniden yüklendi.",
        }
    }

    fn headers(self) -> [&'static str; 9] {
        match self {
            Self::English => [
                "SUBJECT",
                "TOPIC",
                "Q",
                "C",
                "W",
                "B",
                "SUCCESS",
                "LAST TESTED",
                "LAST RESULT",
            ],
            Self::Turkish => [
                "DERS",
                "KONU",
                "S",
                "D",
                "Y",
                "B",
                "BAŞARI",
                "SON TEST",
                "SON SONUÇ",
            ],
        }
    }

    fn help(self) -> &'static str {
        match self {
            Self::English => "↑/↓ move  / search  e exam  r reload  q quit",
            Self::Turkish => "↑/↓ gezin  / ara  e sınav  r yenile  q çık",
        }
    }

    fn search_title(self) -> &'static str {
        match self {
            Self::English => "Search",
            Self::Turkish => "Arama",
        }
    }

    fn search_prompt(self, search: &str) -> String {
        match self {
            Self::English => {
                format!("Search (Enter applies, Esc cancels):\n{search}")
            }
            Self::Turkish => {
                format!("Arama (Enter uygular, Esc iptal eder):\n{search}")
            }
        }
    }
}

fn is_turkish_locale(locale: Option<&str>) -> bool {
    locale.is_some_and(|locale| {
        let locale = locale.to_ascii_lowercase().replace('_', "-");
        locale == "tr" || locale.starts_with("tr-")
    })
}

struct App {
    tracker: Tracker,
    language: Language,
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
        language: Language::from_system(),
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
            app.message = app.language.loaded(app.tracker.exam.label());
        }
        KeyCode::Char('/') => app.search_input = Some(app.query.clone()),
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.tracker = Tracker::load(app.tracker.exam)?;
            app.selected = 0;
            app.message = app.language.reload_message().into();
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
    let matches = |item: &Value| {
        ["name", "name_en"].iter().any(|key| {
            item.get(*key)
                .and_then(Value::as_str)
                .is_some_and(|name| name.to_lowercase().contains(&query))
        })
    };
    let mut entries = Vec::new();
    for (subject_index, subject) in app.tracker.topics["subjects"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        for (topic_index, topic) in subject["topics"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            if query.is_empty() || matches(subject) || matches(topic) {
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
        Paragraph::new(app.language.title(exam, stats.total)),
        vertical[0],
    );
    let total_q = stats.correct + stats.wrong + stats.blank;
    let success_str = if total_q > 0 {
        format!("{:.1}%", (stats.correct as f64 / total_q as f64) * 100.0)
    } else {
        "—".to_string()
    };
    frame.render_widget(
        Paragraph::new(app.language.summary(
            total_q,
            stats.correct,
            stats.wrong,
            stats.blank,
            &success_str,
        )),
        vertical[1],
    );
    let search = app.search_input.as_deref().unwrap_or(&app.query);
    frame.render_widget(Paragraph::new(app.language.search(search)), vertical[2]);
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
                Cell::from(app.language.name(subject).to_owned()),
                Cell::from(app.language.name(topic).to_owned()),
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
        .header(Row::new(app.language.headers()))
        .block(Block::default().borders(Borders::ALL))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(table, vertical[4], &mut table_state);

    frame.render_widget(Paragraph::new(app.language.help()), vertical[5]);
    if app.search_input.is_some() {
        let popup = centered_rect(60, 20, area);
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(app.language.search_prompt(search))
                .block(
                    Block::default()
                        .title(app.language.search_title())
                        .borders(Borders::ALL),
                )
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_turkish_locales() {
        assert!(is_turkish_locale(Some("tr")));
        assert!(is_turkish_locale(Some("tr-TR")));
        assert!(is_turkish_locale(Some("tr_TR.UTF-8")));
        assert!(!is_turkish_locale(Some("en-US")));
        assert!(!is_turkish_locale(None));
    }

    #[test]
    fn localizes_labels_and_names() {
        let item = json!({"name": "Türkçe", "name_en": "Turkish"});
        assert_eq!(Language::English.name(&item), "Turkish");
        assert_eq!(Language::Turkish.name(&item), "Türkçe");
        assert!(Language::English.title("TYT", 3).contains("topics"));
        assert!(Language::Turkish.title("TYT", 3).contains("konu"));
        assert!(
            Language::English
                .summary(1, 1, 0, 0, "100.0%")
                .contains("Success")
        );
        assert!(
            Language::Turkish
                .summary(1, 1, 0, 0, "100.0%")
                .contains("Başarı")
        );
        assert!(Language::English.help().contains("quit"));
        assert!(Language::Turkish.help().contains("çık"));
        assert!(
            Language::Turkish
                .search_prompt("a")
                .contains("Enter uygular")
        );
    }

    #[test]
    fn falls_back_to_turkish_name_when_translation_is_missing() {
        let item = json!({"name": "Türkçe"});
        assert_eq!(Language::English.name(&item), "Türkçe");
    }
}
