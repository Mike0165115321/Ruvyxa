//! Terminal presentation for the Ruvyxa CLI.
//!
//! Everything here writes to stdout or returns a `String` meant for a human:
//! progress bars, box-drawn tables, ANSI colouring, and the byte/duration
//! formatting the build summary uses. Nothing here decides anything. Keeping it
//! separate is what stops presentation details from being edited in the same
//! file as build logic.
//!
//! Colour has three opt-outs, all in [`paint`]: a non-TTY stdout, `NO_COLOR`,
//! and `TERM=dumb`. Progress bars are TTY-only for the same reason, so CI logs
//! and piped output stay clean.

use std::io::IsTerminal;
use std::path::Path;
use std::time::Duration;

use chrono::Local;

use crate::commands::BenchmarkResult;

pub(crate) fn print_field(name: &str, value: String) {
    let padding = spaces(22, name.len());
    println!("  {}{} {}", label(name), padding, value);
}

pub(crate) const PROGRESS_BAR_WIDTH: usize = 26;

/// Redraws an in-place progress bar on the current line. TTY-only: silent when
/// stdout is not a terminal so CI logs and pipes stay clean.
pub(crate) fn draw_progress_bar(enabled: bool, name: &str, done: usize, total: usize) {
    use std::io::Write;
    if !enabled || total == 0 || !std::io::stdout().is_terminal() {
        return;
    }
    let filled = (PROGRESS_BAR_WIDTH * done.min(total)) / total;
    print!(
        "\r  {} {}{} {}{} {}/{} ",
        dim("◌"),
        label(name),
        spaces(18, name.len()),
        accent("█".repeat(filled)),
        dim("░".repeat(PROGRESS_BAR_WIDTH - filled)),
        done,
        total
    );
    let _ = std::io::stdout().flush();
}

/// Clears a bar drawn by `draw_progress_bar` so the phase line replaces it.
pub(crate) fn clear_progress_bar(enabled: bool) {
    use std::io::Write;
    if !enabled || !std::io::stdout().is_terminal() {
        return;
    }
    print!("\r{}\r", " ".repeat(60 + PROGRESS_BAR_WIDTH));
    let _ = std::io::stdout().flush();
}

pub(crate) fn print_benchmark_table(
    samples: usize,
    results: &[BenchmarkResult],
    root: &Path,
    app_dir: &Path,
    elapsed: Duration,
) {
    print_tui_header(format!("Benchmark ({samples} sample(s))"));
    print_field("root", path_text(root));
    print_field("app dir", path_text(app_dir));
    print_field("scenarios", accent(results.len().to_string()));
    print_field("duration", accent(format_duration(elapsed)));
    println!();

    let rows = results
        .iter()
        .map(|result| {
            [
                result.name.clone(),
                format!("{:.2}ms", result.min_ms),
                format!("{:.2}ms", result.median_ms),
                format!("{:.2}ms", result.avg_ms),
                format!("{:.2}ms", result.max_ms),
            ]
        })
        .collect::<Vec<_>>();
    let headers = ["Scenario", "Min", "Median", "Avg", "Max"];
    let widths = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            rows.iter()
                .map(|row| row[index].len())
                .max()
                .unwrap_or(0)
                .max(header.len())
        })
        .collect::<Vec<_>>();

    print_table_separator(&widths);
    print_box_row(
        headers,
        [
            label(headers[0]),
            label(headers[1]),
            label(headers[2]),
            label(headers[3]),
            label(headers[4]),
        ],
        &widths,
        1,
    );
    print_table_separator(&widths);

    for row in rows {
        print_box_row(
            [&row[0], &row[1], &row[2], &row[3], &row[4]],
            [
                accent(&row[0]),
                ok_text(&row[1]),
                ok_text(&row[2]),
                ok_text(&row[3]),
                ok_text(&row[4]),
            ],
            &widths,
            1,
        );
    }
    print_table_separator(&widths);
}

pub(crate) fn print_tui_header(title: impl AsRef<str>) {
    println!("\n{}", heading(tui_header_title(title)));
    println!();
    print_field("time", accent(current_timestamp()));
}

pub(crate) fn tui_header_title(title: impl AsRef<str>) -> String {
    format!("🦊 Ruvyxa {}", title.as_ref())
}

pub(crate) fn print_table_separator(widths: &[usize]) {
    print!("  {}", dim("+"));
    for width in widths {
        print!("{}", dim("-".repeat(*width + 2)));
        print!("{}", dim("+"));
    }
    println!();
}

/// Prints one bordered table row. Columns whose index is at least
/// `right_align_from` are right-aligned (numeric columns); earlier columns are
/// left-aligned (text columns).
pub(crate) fn print_box_row<const N: usize>(
    raw: [&str; N],
    styled: [String; N],
    widths: &[usize],
    right_align_from: usize,
) {
    print!("  {}", dim("|"));
    for index in 0..N {
        if index < right_align_from {
            print!(
                " {}{} {}",
                styled[index],
                spaces(widths[index], raw[index].len()),
                dim("|")
            );
        } else {
            print!(
                " {}{} {}",
                spaces(widths[index], raw[index].len()),
                styled[index],
                dim("|")
            );
        }
    }
    println!();
}

pub(crate) fn spaces(width: usize, len: usize) -> String {
    " ".repeat(width.saturating_sub(len))
}

pub(crate) fn current_timestamp() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub(crate) fn format_duration(duration: Duration) -> String {
    if duration.as_secs() > 0 {
        format!("{:.2}s", duration.as_secs_f64())
    } else {
        format!("{:.0}ms", duration.as_secs_f64() * 1000.0)
    }
}

pub(crate) fn format_bytes(bytes: usize) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;

    if bytes < KIB as usize {
        return format!("{bytes} B");
    }

    let kibibytes = bytes as f64 / KIB;
    if bytes < MIB as usize {
        return if kibibytes < 10.0 {
            format!("{kibibytes:.1} kB")
        } else {
            format!("{kibibytes:.0} kB")
        };
    }

    let mebibytes = bytes as f64 / MIB;
    if mebibytes < 10.0 {
        format!("{mebibytes:.1} MB")
    } else {
        format!("{mebibytes:.0} MB")
    }
}

pub(crate) fn heading(value: impl AsRef<str>) -> String {
    paint(value, "1;35")
}

pub(crate) fn label(value: impl AsRef<str>) -> String {
    paint(value, "90")
}

pub(crate) fn accent(value: impl AsRef<str>) -> String {
    paint(value, "36")
}

pub(crate) fn dim(value: impl AsRef<str>) -> String {
    paint(value, "90")
}

pub(crate) fn ok_text(value: impl AsRef<str>) -> String {
    paint(value, "32")
}

pub(crate) fn warn_text(value: impl AsRef<str>) -> String {
    paint(value, "33")
}

pub(crate) fn error_label() -> String {
    paint("[error]", "31")
}

pub(crate) fn alert_text(value: impl AsRef<str>) -> String {
    paint(value, "31")
}

pub(crate) fn success() -> String {
    ok_text("[ok]")
}

pub(crate) fn path_text(path: &Path) -> String {
    paint(path.display().to_string(), "34")
}

pub(crate) fn display_path_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub(crate) fn exists_status(path: &Path) -> String {
    if path.exists() {
        ok_text("ok")
    } else {
        warn_text("missing")
    }
}

pub(crate) fn tool_status(value: String) -> String {
    if value == "missing" {
        warn_text(value)
    } else {
        ok_text(value)
    }
}

pub(crate) fn compatibility_status(value: String) -> String {
    if value.starts_with("ok ") {
        ok_text(value)
    } else {
        warn_text(value)
    }
}

pub(crate) fn paint(value: impl AsRef<str>, code: &str) -> String {
    let value = value.as_ref();
    if !std::io::stdout().is_terminal() {
        return value.to_string();
    }

    if std::env::var_os("NO_COLOR").is_some() {
        return value.to_string();
    }

    if std::env::var("TERM")
        .map(|term| term.eq_ignore_ascii_case("dumb"))
        .unwrap_or(false)
    {
        return value.to_string();
    }

    format!("\x1b[{code}m{value}\x1b[0m")
}
