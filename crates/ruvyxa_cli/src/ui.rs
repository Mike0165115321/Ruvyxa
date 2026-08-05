//! Terminal presentation for the Ruvyxa CLI.
//!
//! Everything shared with the dev server — colour, field layout, tables, the
//! progress track, the mascot, byte and duration formatting — lives in
//! `ruvyxa_tui` and is re-exported here so call sites keep reading `accent(..)`
//! rather than a crate path. What stays in this file is presentation only the
//! CLI has: the command header and the tables specific to `bench`, `doctor`,
//! and `check`.
//!
//! Nothing here decides anything. Keeping it separate is what stops
//! presentation details from being edited in the same file as build logic.

use std::path::Path;
use std::time::Duration;

// Re-exported rather than imported: sibling modules reach these through
// `crate::*`, and a plain `use` would keep the names private to this file.
use ruvyxa_tui::badge;
pub(crate) use ruvyxa_tui::{
    Spinner, accent, alert_text, bar, brand, clear_progress_bar, column_widths, current_timestamp,
    dim, display_path_relative, draw_progress_bar, error_label, exists_status, format_bytes,
    format_duration, heading, info, label, note, number, ok_text, path_text, print_box_row,
    print_field, print_phase, print_section, print_table_separator, spaces, success,
    tui_header_title, warn_text,
};

use crate::commands::BenchmarkResult;

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
    print_field("scenarios", number(results.len().to_string()));
    print_field("duration", accent(format_duration(elapsed)));
    println!();

    // Scenarios in one run differ by orders of magnitude — route discovery in
    // milliseconds against a production build in seconds. The bar is scaled to
    // the slowest median so the shape of that gap is visible before the numbers
    // are read.
    let slowest_median = results
        .iter()
        .map(|result| result.median_ms)
        .fold(0.0_f64, f64::max);
    let rows = results
        .iter()
        .map(|result| {
            [
                result.name.clone(),
                format!("{:.2}ms", result.min_ms),
                format!("{:.2}ms", result.median_ms),
                format!("{:.2}ms", result.avg_ms),
                format!("{:.2}ms", result.max_ms),
                bar(result.median_ms, slowest_median, BENCHMARK_BAR_WIDTH),
            ]
        })
        .collect::<Vec<_>>();
    let headers = ["Scenario", "Min", "Median", "Avg", "Max", "Median share"];
    let widths = column_widths(&headers, &rows);

    print_table_separator(&widths);
    print_box_row(
        headers,
        [
            label(headers[0]),
            label(headers[1]),
            label(headers[2]),
            label(headers[3]),
            label(headers[4]),
            label(headers[5]),
        ],
        &widths,
        BENCHMARK_ALIGNMENT,
    );
    print_table_separator(&widths);

    for row in rows {
        print_box_row(
            [&row[0], &row[1], &row[2], &row[3], &row[4], &row[5]],
            [
                accent(&row[0]),
                ok_text(&row[1]),
                number(&row[2]),
                info(&row[3]),
                warn_text(&row[4]),
                note(&row[5]),
            ],
            &widths,
            BENCHMARK_ALIGNMENT,
        );
    }
    print_table_separator(&widths);
}

const BENCHMARK_BAR_WIDTH: usize = 12;

/// Scenario name left, the four timings right, and the bar left again so every
/// bar grows from the same edge.
const BENCHMARK_ALIGNMENT: [bool; 6] = [false, true, true, true, true, false];

pub(crate) fn print_tui_header(title: impl AsRef<str>) {
    let title = title.as_ref();
    let badge = badge(title);
    println!("\n{}", heading(tui_header_title(title)));
    println!("  {} {}", badge.icon, dim(badge.tagline));
    println!();
    print_field("time", dim(current_timestamp()));
}

/// The line a command ends on when it succeeded. The mascot appears here and
/// nowhere else in a result, so a finished run is recognisable by shape from
/// across the room.
pub(crate) fn print_success_banner(message: impl AsRef<str>, duration: Duration) {
    println!(
        "\n  {} {} {}\n",
        brand("🦊"),
        ok_text(message.as_ref()),
        dim(format!("· {}", format_duration(duration)))
    );
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
