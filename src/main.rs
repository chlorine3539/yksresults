mod data;
#[path = "okulizyon-importer.rs"]
mod okulizyon_importer;
mod report;
mod tui;

use data::{Exam, Tracker};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("yksresults: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("tui") => tui::run(),
        Some("report") => {
            let key = args
                .get(1)
                .ok_or("Usage: yksresults report <tyt|ayt> [--write]")?;
            let exam = Exam::from_key(key).ok_or("Exam must be tyt or ayt")?;
            let write = args.iter().any(|arg| arg == "--write");
            report::print_report(&Tracker::load(exam)?, write)
        }
        Some("import-tyt") => run_import(&args, Exam::Tyt),
        Some("import-ayt") => run_import(&args, Exam::Ayt),
        Some("--help" | "-h" | "help") => {
            print_help();
            Ok(())
        }
        Some("--version" | "-V") => {
            println!("yksresults {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some(command) => {
            Err(format!("unknown command: {command}\n\nRun `yksresults --help` for usage.").into())
        }
    }
}

fn flag_value<'a>(
    args: &'a [String],
    flag: &str,
) -> Result<Option<&'a str>, Box<dyn std::error::Error>> {
    let Some(index) = args.iter().position(|arg| arg == flag) else {
        return Ok(None);
    };
    let value = args
        .get(index + 1)
        .map(String::as_str)
        .ok_or_else(|| format!("Missing value for {flag}"))?;
    Ok(Some(value))
}

fn run_import(args: &[String], exam: Exam) -> Result<(), Box<dyn std::error::Error>> {
    let date = flag_value(args, "--date")?;
    let file = flag_value(args, "--file")?.map(Path::new);
    let tracker = Tracker::load(exam)?;
    okulizyon_importer::import(&tracker, date, file)?;
    Ok(())
}

fn print_help() {
    println!(
        "yksresults - Terminal YKS results tracker\n\n\
         Usage:\n  yksresults [tui]\n  yksresults report <tyt|ayt> [--write]\n  yksresults import-tyt [--file PATH] [--date YYYY-MM-DD]\n  yksresults import-ayt [--file PATH] [--date YYYY-MM-DD]\n\n\
         Commands:\n  tui          Open the interactive topic tracker (default)\n  report       Print a topic progress report\n  import-tyt   Import a TYT Okulizyon HTML export\n  import-ayt   Import an AYT Okulizyon HTML export\n\n\
         Options:\n  --file PATH  Import and archive one HTML file instead of scanning the results folder\n  --date DATE  Set the rehearsal date when the filename has none\n"
    );
}

#[cfg(test)]
mod tests {
    use super::flag_value;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn reads_optional_flag_values() {
        assert_eq!(flag_value(&args(&["import-ayt"]), "--file").unwrap(), None);
        assert_eq!(
            flag_value(&args(&["import-ayt", "--file", "result.html"]), "--file").unwrap(),
            Some("result.html")
        );
        assert_eq!(
            flag_value(
                &args(&["import-ayt", "--file", "a.html", "--date", "2026-11-10"]),
                "--date"
            )
            .unwrap(),
            Some("2026-11-10")
        );
    }

    #[test]
    fn rejects_flag_without_value() {
        assert!(flag_value(&args(&["import-ayt", "--file"]), "--file").is_err());
    }
}
