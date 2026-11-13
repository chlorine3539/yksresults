use crate::data::Tracker;
use serde_json::Value;

fn number(value: Option<&Value>, default: i64) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(default)
}

fn text(value: Option<&Value>, default: &str) -> String {
    value.and_then(Value::as_str).unwrap_or(default).to_owned()
}

/// Weakness score based purely on rehearsal statistics.
/// Wrong answers are weighted more heavily than blanks.
pub fn weakness(topic: &Value) -> f64 {
    3.0 * number(topic.get("wrong_total"), 0) as f64
        + 2.0 * number(topic.get("blank_total"), 0) as f64
}

pub fn render(data: &Value, exam_name: &str) -> String {
    let subjects: Vec<&Value> = data
        .get("subjects")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect();
    let mut lines = vec![format!("{exam_name} Topic Tracker"), String::new()];
    let name_width = subjects
        .iter()
        .map(|subject| text(subject.get("name"), "").chars().count())
        .max()
        .unwrap_or(0)
        .max("Subject".len());
    let header = format!(
        "{:<name_width$} {:>6} {:>9} {:>7} {:>6} {:>6} {:>8}",
        "Subject", "Topics", "Questions", "Correct", "Wrong", "Blank", "Success"
    );
    lines.push(header.clone());
    lines.push("-".repeat(header.chars().count()));

    let mut total_correct = 0i64;
    let mut total_wrong = 0i64;
    let mut total_blank = 0i64;
    let mut total_topics = 0usize;
    // (score, subject_name, topic_name, correct, wrong, blank)
    let mut weak_topics: Vec<(f64, String, String, i64, i64, i64)> = Vec::new();

    for subject in subjects {
        let name = text(subject.get("name"), "");
        let topics: Vec<&Value> = subject
            .get("topics")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .collect();
        let mut correct = 0i64;
        let mut wrong = 0i64;
        let mut blank = 0i64;
        for topic in &topics {
            let topic_correct = number(topic.get("correct_total"), 0);
            let topic_wrong = number(topic.get("wrong_total"), 0);
            let topic_blank = number(topic.get("blank_total"), 0);
            correct += topic_correct;
            wrong += topic_wrong;
            blank += topic_blank;
            let score = weakness(topic);
            if score > 0.0 {
                weak_topics.push((
                    score,
                    name.clone(),
                    text(topic.get("name"), ""),
                    topic_correct,
                    topic_wrong,
                    topic_blank,
                ));
            }
        }
        let total_q = correct + wrong + blank;
        let success_rate = if total_q > 0 {
            format!("{:.1}%", (correct as f64 / total_q as f64) * 100.0)
        } else {
            "—".to_string()
        };
        lines.push(format!(
            "{name:<name_width$} {:>6} {:>9} {:>7} {:>6} {:>6} {:>8}",
            topics.len(),
            total_q,
            correct,
            wrong,
            blank,
            success_rate
        ));
        total_correct += correct;
        total_wrong += wrong;
        total_blank += blank;
        total_topics += topics.len();
    }

    lines.push("-".repeat(header.chars().count()));
    let total_q = total_correct + total_wrong + total_blank;
    let total_success_rate = if total_q > 0 {
        format!("{:.1}%", (total_correct as f64 / total_q as f64) * 100.0)
    } else {
        "—".to_string()
    };
    lines.push(format!(
        "{:<name_width$} {:>6} {:>9} {:>7} {:>6} {:>6} {:>8}",
        "TOTAL", total_topics, total_q, total_correct, total_wrong, total_blank, total_success_rate
    ));
    lines.push(String::new());

    weak_topics.sort_by(|left, right| {
        right
            .0
            .total_cmp(&left.0)
            .then_with(|| left.2.cmp(&right.2))
    });
    if weak_topics.is_empty() {
        lines.push("No rehearsal data yet — import an Okulizyon result to see weak topics.".into());
    } else {
        lines.push("Weak topics (by wrong + blank count)".into());
        for (score, subject, topic, correct, wrong, blank) in weak_topics.into_iter().take(12) {
            let total_q = correct + wrong + blank;
            let rate_str = if total_q > 0 {
                format!("{:.1}%", (correct as f64 / total_q as f64) * 100.0)
            } else {
                "—".to_string()
            };
            lines.push(format!(
                "  [{score:>4.1}] {topic} ({subject}) — questions={total_q}, success={rate_str}, correct={correct}, wrong={wrong}, blank={blank}"
            ));
        }
    }
    lines.join("\n") + "\n"
}

pub fn print_report(tracker: &Tracker, write: bool) -> Result<(), Box<dyn std::error::Error>> {
    let exam_name = tracker.exam.label();
    let report = render(&tracker.topics, exam_name);
    print!("{report}");
    if write {
        let dashboard = format!("# {exam_name} Topic Dashboard\n\n```text\n{report}```\n");
        crate::data::atomic_write(
            &tracker.directory.join("dashboard.md"),
            dashboard.as_bytes(),
        )?;
        println!(
            "\nWrote {}",
            tracker.directory.join("dashboard.md").display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reports_wrong_blank_totals() {
        let data = json!({
            "subjects": [{
                "name": "Mathematics",
                "topics": [
                    {"name":"Algebra","correct_total":1,"wrong_total":2,"blank_total":1},
                    {"name":"Geometry","correct_total":0,"wrong_total":0,"blank_total":0}
                ]
            }]
        });
        let report = render(&data, "TYT");
        assert!(report.contains("TYT Topic Tracker"));
        assert!(report.contains("Mathematics"));
        assert!(report.contains("Weak topics"));
        assert!(report.contains("Algebra"));
        assert!(report.contains("Questions"));
        assert!(report.contains("Success"));
    }

    #[test]
    fn long_subject_names_keep_columns_aligned() {
        let data = json!({
            "subjects": [{
                "name": "Religious Culture & Moral Knowledge",
                "topics": [{"name":"Faith","correct_total":1,"wrong_total":0,"blank_total":0}]
            }]
        });
        let report = render(&data, "TYT");
        let widths: Vec<usize> = report
            .lines()
            .skip(2)
            .take(5)
            .map(|line| line.chars().count())
            .collect();
        assert!(
            widths.windows(2).all(|pair| pair[0] == pair[1]),
            "{widths:?}"
        );
    }

    #[test]
    fn weakness_score_uses_only_stats() {
        let topic = json!({"wrong_total": 2, "blank_total": 1});
        // 3*2 + 2*1 = 8.0
        assert!((weakness(&topic) - 8.0).abs() < f64::EPSILON);
    }
}
