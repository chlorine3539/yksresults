use serde_json::Value;
use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Exam {
    Tyt,
    Ayt,
}

impl Exam {
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "tyt" => Some(Self::Tyt),
            "ayt" => Some(Self::Ayt),
            _ => None,
        }
    }

    pub fn directory_name(self) -> &'static str {
        match self {
            Self::Tyt => "tyt-topic-tracker",
            Self::Ayt => "ayt-topic-tracker",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Tyt => "TYT",
            Self::Ayt => "AYT",
        }
    }

    fn default_topics(self) -> &'static str {
        match self {
            Self::Tyt => include_str!("../topics/tyt.json"),
            Self::Ayt => include_str!("../topics/ayt.json"),
        }
    }
}

pub struct Tracker {
    pub exam: Exam,
    pub directory: PathBuf,
    pub topics: Value,
}

impl Tracker {
    pub fn load(exam: Exam) -> Result<Self, Box<dyn Error>> {
        let application_directory = data_home()?.join("yks-tracker");
        ensure_private_dir(&application_directory)?;
        let directory = application_directory.join(exam.directory_name());
        if exam == Exam::Ayt && !directory.exists() {
            let legacy = application_directory.join("ayt-quantitative-topic-tracker");
            if legacy.exists() {
                let _ = fs::rename(&legacy, &directory);
            }
        }
        ensure_private_dir(&directory)?;
        for child in ["results", "analysis"] {
            ensure_private_dir(&directory.join(child))?;
        }

        let topics_path = directory.join("topics.json");
        initialize_file(&topics_path, exam.default_topics().as_bytes())?;

        let mut topics = read_json(&topics_path)?;
        let defaults: Value = serde_json::from_str(exam.default_topics())?;
        let mut modified = merge_defaults(&mut topics, &defaults);
        if exam == Exam::Tyt {
            modified |= prune_retired_topics(&mut topics);
        }
        if modified {
            let mut bytes = serde_json::to_vec_pretty(&topics)?;
            bytes.push(b'\n');
            atomic_write(&topics_path, &bytes)?;
        }

        Ok(Self {
            exam,
            topics,
            directory,
        })
    }
}

/// Adds subjects and topics that exist in `defaults` but not in `data` (matched by `id`),
/// and syncs subject and topic display names. Existing statistics are never modified.
/// Returns whether anything was modified.
fn merge_defaults(data: &mut Value, defaults: &Value) -> bool {
    let (Some(subjects), Some(default_subjects)) = (
        data.get_mut("subjects").and_then(Value::as_array_mut),
        defaults.get("subjects").and_then(Value::as_array),
    ) else {
        return false;
    };
    let mut modified = false;
    for default_subject in default_subjects {
        let Some(id) = default_subject.get("id").and_then(Value::as_str) else {
            continue;
        };
        let existing = subjects
            .iter_mut()
            .find(|subject| subject.get("id").and_then(Value::as_str) == Some(id));
        let Some(subject) = existing else {
            subjects.push(default_subject.clone());
            modified = true;
            continue;
        };
        if let Some(def_name) = default_subject.get("name").and_then(Value::as_str)
            && subject.get("name").and_then(Value::as_str) != Some(def_name)
        {
            subject["name"] = Value::String(def_name.to_string());
            modified = true;
        }
        let Some(default_topics) = default_subject.get("topics").and_then(Value::as_array) else {
            continue;
        };
        let Some(topics) = subject.get_mut("topics").and_then(Value::as_array_mut) else {
            continue;
        };
        for default_topic in default_topics {
            let Some(topic_id) = default_topic.get("id").and_then(Value::as_str) else {
                continue;
            };
            if let Some(existing_topic) = topics
                .iter_mut()
                .find(|topic| topic.get("id").and_then(Value::as_str) == Some(topic_id))
            {
                if let Some(def_name) = default_topic.get("name").and_then(Value::as_str)
                    && existing_topic.get("name").and_then(Value::as_str) != Some(def_name)
                {
                    existing_topic["name"] = Value::String(def_name.to_string());
                    modified = true;
                }
            } else {
                topics.push(default_topic.clone());
                modified = true;
            }
        }
    }
    modified
}

/// Drops retired TYT topics (see `importer::is_retired_tyt_topic`) that never hold any data.
fn prune_retired_topics(data: &mut Value) -> bool {
    let mut removed = false;
    let subjects = data.get_mut("subjects").and_then(Value::as_array_mut);
    for subject in subjects.into_iter().flatten() {
        let Some(topics) = subject.get_mut("topics").and_then(Value::as_array_mut) else {
            continue;
        };
        let before = topics.len();
        topics.retain(|topic| {
            let id = topic.get("id").and_then(Value::as_str).unwrap_or("");
            let has_data = topic
                .get("question_total")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > 0;
            has_data || !crate::importer::is_retired_tyt_topic(id)
        });
        removed |= topics.len() != before;
    }
    removed
}

pub fn data_home() -> Result<PathBuf, Box<dyn Error>> {
    match env::var_os("XDG_DATA_HOME") {
        Some(path) if !path.is_empty() => {
            let path = PathBuf::from(path);
            if !path.is_absolute() {
                return Err("XDG_DATA_HOME must be an absolute path".into());
            }
            Ok(path)
        }
        _ => {
            let home = env::var_os("HOME").ok_or("HOME is not set")?;
            Ok(PathBuf::from(home).join(".local/share"))
        }
    }
}

pub fn ensure_private_dir(path: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

pub fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

/// Writes `contents` to a fresh private temporary file next to `path` and returns its path.
fn write_temporary(path: &Path, contents: &[u8]) -> Result<PathBuf, Box<dyn Error>> {
    let parent = path.parent().ok_or("file path has no parent directory")?;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let file_name = path
        .file_name()
        .ok_or("file path has no name")?
        .to_string_lossy();
    let temporary = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        timestamp
    ));
    let result = (|| -> Result<(), Box<dyn Error>> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(temporary)
}

pub fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), Box<dyn Error>> {
    let parent = path.parent().ok_or("file path has no parent directory")?;
    ensure_private_dir(parent)?;
    let temporary = write_temporary(path, contents)?;
    let result = fs::rename(&temporary, path);
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    Ok(result?)
}

fn initialize_file(path: &Path, contents: &[u8]) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        return Ok(());
    }
    let temporary = write_temporary(path, contents)?;
    let result = match fs::hard_link(&temporary, path) {
        Ok(()) => Ok(()),
        Err(_error) if path.exists() => Ok(()),
        Err(error) => Err(error.into()),
    };
    let _ = fs::remove_file(&temporary);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_adds_missing_subjects_and_topics_without_touching_stats() {
        let mut data = json!({"subjects": [
            {"id": "a", "topics": [{"id": "a1", "correct_total": 5}]}
        ]});
        let defaults = json!({"subjects": [
            {"id": "a", "topics": [{"id": "a1", "correct_total": 0}, {"id": "a2", "correct_total": 0}]},
            {"id": "b", "topics": [{"id": "b1"}]}
        ]});
        assert!(merge_defaults(&mut data, &defaults));
        assert_eq!(data["subjects"][0]["topics"][0]["correct_total"], 5);
        assert_eq!(data["subjects"][0]["topics"][1]["id"], "a2");
        assert_eq!(data["subjects"][1]["id"], "b");
        assert!(!merge_defaults(&mut data, &defaults));
    }

    #[test]
    fn merge_syncs_updated_names_without_touching_stats() {
        let mut data = json!({"subjects": [
            {
                "id": "turkce",
                "name": "Turkish",
                "topics": [{"id": "t1", "name": "Word Meaning", "correct_total": 42}]
            }
        ]});
        let defaults = json!({"subjects": [
            {
                "id": "turkce",
                "name": "Türkçe",
                "topics": [{"id": "t1", "name": "Sözcükte Anlam", "correct_total": 0}]
            }
        ]});
        assert!(merge_defaults(&mut data, &defaults));
        assert_eq!(data["subjects"][0]["name"], "Türkçe");
        assert_eq!(data["subjects"][0]["topics"][0]["name"], "Sözcükte Anlam");
        assert_eq!(data["subjects"][0]["topics"][0]["correct_total"], 42);
        assert!(!merge_defaults(&mut data, &defaults));
    }

    #[test]
    fn bundled_topics_have_unique_ids() {
        for exam in [Exam::Tyt, Exam::Ayt] {
            let data: Value = serde_json::from_str(exam.default_topics()).unwrap();
            let mut seen = std::collections::HashSet::new();
            for subject in data["subjects"].as_array().unwrap() {
                assert!(seen.insert(subject["id"].as_str().unwrap().to_owned()));
                for topic in subject["topics"].as_array().unwrap() {
                    assert!(seen.insert(topic["id"].as_str().unwrap().to_owned()));
                }
            }
        }
    }
}
