use std::time::Duration;

use crate::*;

fn env_from<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + use<'a> {
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.to_string())
    }
}

#[test]
fn a_terminal_gets_colour_animation_and_unicode() {
    let capabilities = detect_capabilities(true, env_from(&[]));
    assert_eq!(
        capabilities,
        Capabilities {
            color: true,
            animate: true,
            unicode: true
        }
    );
}

#[test]
fn a_pipe_gets_no_colour_and_no_movement_but_keeps_its_glyphs() {
    assert_eq!(
        detect_capabilities(false, env_from(&[])),
        Capabilities {
            color: false,
            animate: false,
            unicode: true
        }
    );
}

#[test]
fn no_color_keeps_animation_but_drops_colour() {
    let capabilities = detect_capabilities(true, env_from(&[("NO_COLOR", "1")]));
    assert!(!capabilities.color);
    assert!(capabilities.animate);
}

#[test]
fn dumb_terminals_get_nothing() {
    assert_eq!(
        detect_capabilities(true, env_from(&[("TERM", "dumb")])),
        Capabilities::PLAIN
    );
}

#[test]
fn ruvyxa_fun_off_keeps_colour_and_stops_movement() {
    for value in ["0", "false", "off", "no", ""] {
        let capabilities = detect_capabilities(true, env_from(&[("RUVYXA_FUN", value)]));
        assert!(
            capabilities.color,
            "colour should survive RUVYXA_FUN={value}"
        );
        assert!(
            !capabilities.animate,
            "RUVYXA_FUN={value} should stop movement"
        );
    }
}

#[test]
fn ruvyxa_fun_on_leaves_animation_enabled() {
    assert!(detect_capabilities(true, env_from(&[("RUVYXA_FUN", "1")])).animate);
}

#[test]
fn ruvyxa_ascii_selects_the_ascii_glyph_set() {
    let capabilities = detect_capabilities(true, env_from(&[("RUVYXA_ASCII", "1")]));
    assert!(!capabilities.unicode);
    assert_eq!(glyphs(capabilities), ASCII_GLYPHS);
}

#[test]
fn paint_wraps_only_when_colour_is_allowed() {
    assert_eq!(paint_when(true, "build", "36"), "\x1b[36mbuild\x1b[0m");
    assert_eq!(paint_when(false, "build", "36"), "build");
}

#[test]
fn the_runner_starts_at_the_left_edge() {
    let (behind, runner, ahead) = runner_cells(UNICODE_GLYPHS, 0, 10, 0);
    assert_eq!(behind, "");
    assert_eq!(runner, "🦊");
    assert_eq!(ahead.chars().count(), TRACK_WIDTH);
}

#[test]
fn the_runner_reaches_the_right_edge_when_the_work_is_done() {
    let (behind, _, ahead) = runner_cells(UNICODE_GLYPHS, 10, 10, 0);
    assert_eq!(behind.chars().count(), TRACK_WIDTH);
    assert_eq!(ahead, "");
}

#[test]
fn the_track_keeps_one_width_at_every_position() {
    for done in 0..=10 {
        let (behind, _, ahead) = runner_cells(UNICODE_GLYPHS, done, 10, 0);
        assert_eq!(
            behind.chars().count() + ahead.chars().count(),
            TRACK_WIDTH,
            "track width changed at {done}/10"
        );
    }
}

#[test]
fn dust_alternates_behind_the_runner_between_frames() {
    let (first, _, _) = runner_cells(UNICODE_GLYPHS, 5, 10, 0);
    let (second, _, _) = runner_cells(UNICODE_GLYPHS, 5, 10, 1);
    assert_ne!(first, second);
    assert!(first.ends_with(UNICODE_GLYPHS.dust[0]));
    assert!(second.ends_with(UNICODE_GLYPHS.dust[1]));
}

#[test]
fn overshooting_the_total_does_not_panic_or_overflow_the_track() {
    let (behind, _, ahead) = runner_cells(UNICODE_GLYPHS, 99, 10, 0);
    assert_eq!(behind.chars().count(), TRACK_WIDTH);
    assert_eq!(ahead, "");
}

#[test]
fn a_zero_total_leaves_the_track_empty() {
    let (behind, _, ahead) = runner_cells(UNICODE_GLYPHS, 3, 0, 0);
    assert_eq!(behind, "");
    assert_eq!(ahead.chars().count(), TRACK_WIDTH);
}

#[test]
fn field_and_phase_lines_put_their_value_in_the_same_column() {
    // The alignment the two widths exist to produce: a field line and a phase
    // line read as one table.
    // Compared in characters, not bytes: the phase status glyph is one column
    // wide but three bytes long.
    fn value_column(line: &str) -> usize {
        line[..line.find("value").expect("value is present")]
            .chars()
            .count()
    }

    assert_eq!(
        value_column(&field_line("app dir", "value".to_string())),
        value_column(&phase_line(
            "routes discovered",
            "value".to_string(),
            Duration::ZERO
        )),
        "field and phase lines disagree on the value column"
    );
}

#[test]
fn header_title_keeps_the_mascot_in_piped_output() {
    assert_eq!(tui_header_title("Build"), "🦊 Ruvyxa Build");
}

#[test]
fn durations_switch_units_at_one_second() {
    assert_eq!(format_duration(Duration::from_millis(120)), "120ms");
    assert_eq!(format_duration(Duration::from_millis(1500)), "1.50s");
}

#[test]
fn byte_sizes_switch_units_at_each_boundary() {
    assert_eq!(format_bytes(512), "512 B");
    assert_eq!(format_bytes(2048), "2.0 kB");
    assert_eq!(format_bytes(2 * 1024 * 1024), "2.0 MB");
}

#[test]
fn a_spinner_on_a_non_animating_terminal_still_reports_its_phase() {
    // No terminal in the test process, so no thread is spawned and `finish`
    // degrades to the plain phase line.
    let spinner = Spinner::start("bundling");
    spinner.finish("12 chunks".to_string());
}
