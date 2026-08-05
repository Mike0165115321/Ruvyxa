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
pub(crate) use ruvyxa_tui::{
    Spinner, accent, alert_text, clear_progress_bar, current_timestamp, dim, display_path_relative,
    draw_progress_bar, error_label, exists_status, format_bytes, format_duration, heading, label,
    ok_text, path_text, print_box_row, print_field, print_phase, print_table_separator, spaces,
    success, tui_header_title, warn_text,
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
