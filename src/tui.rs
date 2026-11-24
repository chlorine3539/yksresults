use crate::data::{Exam, Tracker};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Block, Borders, Cell, Chart, Clear, Dataset, GraphType, Paragraph, Row, Table, TableState,
};
use ratatui::{Terminal, widgets::Wrap};
use serde_json::Value;
use std::fs;
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
            Self::English => "↑/↓ move  / search  Tab tab  e exam  r reload  q quit",
            Self::Turkish => "↑/↓ gezin  / ara  Tab sekme  e sınav  r yenile  q çık",
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

    fn topic_tab(self) -> &'static str {
        match self {
            Self::English => "Topics",
            Self::Turkish => "Konular",
        }
    }

    fn graph_tab(self) -> &'static str {
        match self {
            Self::English => "Graph",
            Self::Turkish => "Grafik",
        }
    }

    fn graph_title(self, latest: Option<f64>) -> String {
        match self {
            Self::English => match latest {
                Some(net) => format!("Net trend — latest {net:.2}"),
                None => "Net trend — no data".to_owned(),
            },
            Self::Turkish => match latest {
                Some(net) => format!("Net eğilimi — son {net:.2}"),
                None => "Net eğilimi — veri yok".to_owned(),
            },
        }
    }

    fn graph_empty(self) -> &'static str {
        match self {
            Self::English => "No imported results yet. Use import-tyt or import-ayt.",
            Self::Turkish => "Henüz içe aktarılmış sonuç yok. import-tyt veya import-ayt kullanın.",
        }
    }

    fn date_axis(self) -> &'static str {
        match self {
            Self::English => "Date",
            Self::Turkish => "Tarih",
        }
    }

    fn net_axis(self) -> &'static str {
        "Net"
    }

    fn net_dataset(self) -> &'static str {
        match self {
            Self::English => "Total net",
            Self::Turkish => "Toplam net",
        }
    }
}

fn is_turkish_locale(locale: Option<&str>) -> bool {
    locale.is_some_and(|locale| {
        let locale = locale.to_ascii_lowercase().replace('_', "-");
        locale == "tr" || locale.starts_with("tr-")
    })
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Tab {
    Topics,
    Graph,
}

#[derive(Clone, Debug, PartialEq)]
struct NetPoint {
    date: String,
    net: f64,
}

struct App {
    tracker: Tracker,
    language: Language,
    tab: Tab,
    net_points: Vec<NetPoint>,
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
    let tracker = Tracker::load(Exam::Tyt)?;
    let net_points = load_net_points(&tracker);
    let mut app = App {
        tracker,
        language: Language::from_system(),
        tab: Tab::Topics,
        net_points,
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
            app.net_points = load_net_points(&app.tracker);
            app.selected = 0;
            app.message = app.language.loaded(app.tracker.exam.label());
        }
        KeyCode::Char('/') => app.search_input = Some(app.query.clone()),
        KeyCode::Tab | KeyCode::BackTab => {
            app.tab = match app.tab {
                Tab::Topics => Tab::Graph,
                Tab::Graph => Tab::Topics,
            };
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.tracker = Tracker::load(app.tracker.exam)?;
            app.net_points = load_net_points(&app.tracker);
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

fn load_net_points(tracker: &Tracker) -> Vec<NetPoint> {
    let Ok(entries) = fs::read_dir(tracker.directory.join("analysis")) else {
        return Vec::new();
    };
    let mut paths: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    paths.sort();
    points_from_values(
        paths
            .iter()
            .filter_map(|path| crate::data::read_json(path).ok()),
    )
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

fn points_from_values(values: impl IntoIterator<Item = Value>) -> Vec<NetPoint> {
    let mut points: Vec<NetPoint> = values
        .into_iter()
        .filter_map(|value| {
            let date = value.get("date")?.as_str()?;
            if !is_iso_date(date) {
                return None;
            }
            let net = value.get("total_net")?.as_f64()?;
            Some(NetPoint {
                date: date.to_owned(),
                net,
            })
        })
        .collect();
    points.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| left.net.total_cmp(&right.net))
    });
    points
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
    let active = Style::default().add_modifier(Modifier::REVERSED);
    let inactive = Style::default().fg(Color::DarkGray);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} ", app.language.topic_tab()),
                if app.tab == Tab::Topics {
                    active
                } else {
                    inactive
                },
            ),
            Span::raw(" | "),
            Span::styled(
                format!(" {} ", app.language.graph_tab()),
                if app.tab == Tab::Graph {
                    active
                } else {
                    inactive
                },
            ),
        ])),
        vertical[3],
    );
    if !app.message.is_empty() {
        frame.render_widget(Paragraph::new(app.message.as_str()), vertical[4]);
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
    if app.tab == Tab::Topics {
        let mut table_state = TableState::default();
        if !entries.is_empty() {
            table_state.select(Some(app.selected));
        }
        let table = Table::new(rows, widths)
            .header(Row::new(app.language.headers()))
            .block(Block::default().borders(Borders::ALL))
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(table, vertical[5], &mut table_state);
    } else {
        draw_graph(frame, app, vertical[5]);
    }

    frame.render_widget(Paragraph::new(app.language.help()), vertical[6]);
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

fn draw_graph(frame: &mut ratatui::Frame<'_>, app: &App, area: ratatui::layout::Rect) {
    let title = app
        .language
        .graph_title(app.net_points.last().map(|point| point.net));
    let block = || Block::default().borders(Borders::ALL).title(title.as_str());
    if app.net_points.is_empty() {
        frame.render_widget(
            Paragraph::new(app.language.graph_empty())
                .block(block())
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }

    let len = app.net_points.len();
    let series: Vec<(f64, f64)> = app
        .net_points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let x = if len == 1 { 0.5 } else { index as f64 };
            (x, point.net)
        })
        .collect();
    let x_bounds = [0.0, if len == 1 { 1.0 } else { (len - 1) as f64 }];
    let min = app
        .net_points
        .iter()
        .map(|point| point.net)
        .fold(0.0, f64::min)
        .min(0.0);
    let max = app
        .net_points
        .iter()
        .map(|point| point.net)
        .fold(0.0, f64::max)
        .max(0.0);
    let padding = ((max - min) * 0.1).max(1.0);
    let y_lower = min - padding;
    let y_upper = max + padding;
    let x_labels = [
        app.net_points.first().unwrap().date.clone(),
        app.net_points[len / 2].date.clone(),
        app.net_points.last().unwrap().date.clone(),
    ];
    let y_labels = [
        format!("{y_lower:.1}"),
        format!("{:.1}", (y_lower + y_upper) / 2.0),
        format!("{y_upper:.1}"),
    ];
    let dataset = Dataset::default()
        .name(app.language.net_dataset())
        .data(&series)
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Cyan));
    let chart = Chart::new(vec![dataset])
        .block(block())
        .x_axis(
            Axis::default()
                .title(app.language.date_axis())
                .bounds(x_bounds)
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .title(app.language.net_axis())
                .bounds([y_lower, y_upper])
                .labels(y_labels),
        );
    frame.render_widget(chart, area);
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

    #[test]
    fn collects_and_sorts_net_points() {
        let points = points_from_values([
            json!({"date": "2026-11-20", "total_net": 30.5}),
            json!({"date": "bad", "total_net": 99.0}),
            json!({"date": "2026-11-10", "total_net": 12.25}),
            json!({"date": "2026-11-15", "total_net": "invalid"}),
        ]);
        assert_eq!(
            points,
            vec![
                NetPoint {
                    date: "2026-11-10".into(),
                    net: 12.25
                },
                NetPoint {
                    date: "2026-11-20".into(),
                    net: 30.5
                },
            ]
        );
    }

    #[test]
    fn localizes_graph_strings() {
        assert!(Language::English.graph_title(Some(42.5)).contains("latest"));
        assert!(Language::Turkish.graph_title(Some(42.5)).contains("son"));
        assert!(Language::English.graph_title(None).contains("no data"));
        assert!(Language::Turkish.graph_title(None).contains("veri yok"));
        assert_eq!(Language::English.topic_tab(), "Topics");
        assert_eq!(Language::Turkish.graph_tab(), "Grafik");
        assert_eq!(Language::English.date_axis(), "Date");
        assert_eq!(Language::Turkish.date_axis(), "Tarih");
    }
}
