# YKS Tracker

YKS Tracker is a native Rust terminal application for viewing TYT and AYT
rehearsal statistics across all subjects and tracks (Sayısal, Eşit Ağırlık, Sözel).
It imports Okulizyon HTML exports and shows, per topic, the question total and the
correct, wrong and blank counts with the calculated success rate, plus a summary across
all subjects. The two exams keep separate inventories and histories.

## Requirements

- Rust 1.88 or newer and Cargo (edition 2024)
- Linux or macOS (Unix only; Windows is not supported)
- A terminal emulator

## Usage

Keys: arrows or `j`/`k` move through topics, `PageUp`/`PageDown` jump by 10,
`/` searches, `e` switches exams, `r` reloads data, and `q` quits. Build an optimized
executable with `cargo build --release`.

CLI commands:

```bash
yks-tracker                 # Launch interactive TUI (same as `yks-tracker tui`)
yks-tracker report tyt      # Print TYT statistics report
yks-tracker report ayt      # Print AYT statistics report
yks-tracker report tyt --write   # Also write dashboard.md to the TYT data directory
yks-tracker import-tyt --file ~/Downloads/tyt-result_10.11.2026.html
yks-tracker import-ayt --file ~/Downloads/ayt-result_10.11.2026.html
yks-tracker --help          # Usage; --version prints the version
```

Without `--file`, `import-tyt` / `import-ayt` re-scan the exam's `results/` directory
and rebuild all statistics from every archived HTML file and analysis.

On first launch, the app initializes missing data files from `topics/`.
It never overwrites existing statistics and automatically adds subjects and topics that
were newly added to `topics/`. Topic IDs that were retired from the bundled lists are
removed from existing data files if they hold no results.

## Application data

Runtime data is stored outside the repository at
`${XDG_DATA_HOME:-~/.local/share}/yks-tracker/`:

- `tyt-topic-tracker/` — TYT topics, results, and analyses
- `ayt-topic-tracker/` — AYT topics, results, and analyses (migrated automatically from legacy `ayt-quantitative-topic-tracker/` if present)

Back up this directory to preserve your study history.

## Topic inventory

The topic lists in `topics/tyt.json` and `topics/ayt.json` were compiled from
the following sources:

| Source | URL |
|---|---|
| MEB Türk Dili ve Edebiyatı Öğretim Programı | meb.gov.tr |
| ÖSYM YKS Kılavuzu | osym.gov.tr |
| ailevegitim.com — TYT/AYT konu listeleri | ailevegitim.com |
| derslig.com — TYT Matematik, Geometri, AYT Geometri konu listeleri | derslig.com |
| erinakademi.com — AYT Sayısal tam konu listesi | erinakademi.com |
| dopinghafiza.com — TYT Geometri & AYT Analitik Geometri konu listeleri | dopinghafiza.com |
| kitapsec.com — TYT Geometri soru dağılımı | kitapsec.com |
| tahtaapp.com — TYT Türkçe, Sosyal Bilimler konu listeleri | tahtaapp.com |
| superprof.com.tr — TYT Fen & Sosyal konu listeleri | superprof.com.tr |
| miletakademi.net — AYT Matematik konu listesi | miletakademi.net |

- **TYT** covers 9 subjects: Turkish (40q), Basic Mathematics & Geometry (40q),
  Physics (7q), Chemistry (7q), Biology (6q), History (5q), Geography (5q),
  Philosophy (5q), and Religious Culture (5q).
- **AYT** covers all 11 subjects across all tracks:
  - *Türk Dili ve Edebiyatı - Sosyal Bilimler-1 (40q):* Türk Dili ve Edebiyatı (24q), Tarih-1 (10q), Coğrafya-1 (6q)
  - *Matematik (40q):* Matematik & Geometri (40q)
  - *Fen Bilimleri (40q):* Fizik (14q), Kimya (13q), Biyoloji (13q)
  - *Sosyal Bilimler-2 (40q):* Tarih-2 (11q), Coğrafya-2 (11q), Felsefe Grubu (12q), Din Kültürü ve Ahlak Bilgisi (6q)

## Okulizyon HTML import

Pass the exported HTML file directly with `--file`. The importer reads it in
place, copies it into the application's `results` directory for archiving,
writes a parsed analysis JSON to `analysis/`, and updates topic statistics.

The rehearsal date is automatically extracted from `DD.MM.YYYY` in the filename
(for example `tyt-result_10.11.2026.html`). If the filename has no date, the import
fails unless you pass `--date YYYY-MM-DD`; the archived copy then gets the date appended:

```bash
yks-tracker import-ayt --file ~/Downloads/ayt-result.html --date 2026-11-10
```

Two exams with the same date and the same name (first 25 characters, slugified) share one
analysis file; the importer refuses to overwrite it with a different HTML file.

AYT analyses combine `Matematik-2` and `Geometri` into the Mathematics section
and compute the total net across all completed AYT sections.

## License

MIT. See [LICENSE](LICENSE).
