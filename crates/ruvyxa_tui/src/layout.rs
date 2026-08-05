//! Field, table, and unit formatting shared by every Ruvyxa command.
//!
//! The two label widths below are chosen together, not independently. A field
//! line is `"  " + label(20) + " "` and a phase line is `"  ◌ " + label(18) +
//! " "`, so both put their value in column 23 and the build summary reads as
//! one table instead of two. Changing one without the other reintroduces the
//! misalignment this module exists to remove.

use std::path::Path;
use std::time::Duration;

use chrono::Local;

use crate::theme::{accent, dim, label, ok_text, paint, warn_text};

/// Width of the label column in a `key: value` field line.
pub const FIELD_LABEL_WIDTH: usize = 20;

/// Width of the label column in a build-phase line, which carries a two-column
/// status glyph before the label.
pub const PHASE_LABEL_WIDTH: usize = 18;

pub fn print_field(name: &str, value: String) {
    println!("{}", field_line(name, value));
}

pub fn field_line(name: &str, value: String) -> String {
    format!(
        "  {}{} {}",
        label(name),
        spaces(FIELD_LABEL_WIDTH, name.len()),
        value
    )
}

pub fn spaces(width: usize, len: usize) -> String {
    " ".repeat(width.saturating_sub(len))
}

pub fn current_timestamp() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn format_duration(duration: Duration) -> String {
    if duration.as_secs() > 0 {
        format!("{:.2}s", duration.as_secs_f64())
    } else {
        format!("{:.0}ms", duration.as_secs_f64() * 1000.0)
    }
}

pub fn format_bytes(bytes: usize) -> String {
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

pub fn print_table_separator(widths: &[usize]) {
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
pub fn print_box_row<const N: usize>(
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

pub fn path_text(path: &Path) -> String {
    paint(path.display().to_string(), "34")
}

pub fn display_path_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub fn exists_status(path: &Path) -> String {
    if path.exists() {
        ok_text("ok")
    } else {
        warn_text("missing")
    }
}

pub fn enabled_text(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}

pub fn accent_count(value: usize) -> String {
    accent(value.to_string())
}
