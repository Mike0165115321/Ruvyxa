//! Terminal capability detection and the single colour palette.
//!
//! Every colour in the Ruvyxa command line resolves to a role named here —
//! `heading`, `accent`, `ok`, `warn`, `alert`, `link`, `label`, `dim`, `brand`.
//! Call sites ask for a role, never for an ANSI code, which is what stops two
//! commands from picking different greens for the same idea.
//!
//! Capability detection runs exactly once per process ([`capabilities`]).
//! [`paint`] used to re-check `is_terminal()` and read two environment
//! variables on every coloured fragment; that was invisible while output was
//! static, but an animated line repaints many fragments per frame, so the
//! answer is cached in a `OnceLock` instead.
//!
//! Three opt-outs are preserved exactly as they were, plus two new ones:
//!
//! - stdout is not a terminal — no colour, no animation
//! - `NO_COLOR` is set — no colour
//! - `TERM=dumb` — no colour, no animation
//! - `RUVYXA_FUN=0` (or `false`, `off`, `no`, empty) — no animation, no mascot
//! - `RUVYXA_ASCII=1` — ASCII glyphs only, for terminals without box drawing

use std::io::IsTerminal;
use std::sync::OnceLock;

/// What the attached terminal can be asked to do. Resolved once per process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    /// ANSI colour escapes are safe to emit.
    pub color: bool,
    /// Carriage-return repainting is safe: a real terminal that has not opted
    /// out. Everything that redraws a line must check this, so piped output and
    /// CI logs keep one line per event.
    pub animate: bool,
    /// Non-ASCII glyphs (box drawing, braille, emoji) are safe to emit.
    pub unicode: bool,
}

impl Capabilities {
    /// Plain, unconditional output: what `TERM=dumb` gets. A pipe differs — it
    /// keeps `unicode`, because a log file renders UTF-8 whatever produced it.
    pub const PLAIN: Self = Self {
        color: false,
        animate: false,
        unicode: false,
    };
}

pub fn capabilities() -> Capabilities {
    static CAPABILITIES: OnceLock<Capabilities> = OnceLock::new();
    *CAPABILITIES.get_or_init(|| {
        detect_capabilities(std::io::stdout().is_terminal(), |name| {
            std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
        })
    })
}

/// The detection rules, taking the terminal answer and the environment as
/// arguments so the decision table can be tested without a terminal.
pub fn detect_capabilities(
    is_terminal: bool,
    env: impl Fn(&str) -> Option<String>,
) -> Capabilities {
    let dumb = env("TERM")
        .map(|term| term.eq_ignore_ascii_case("dumb"))
        .unwrap_or(false);

    Capabilities {
        color: is_terminal && !dumb && env("NO_COLOR").is_none(),
        animate: is_terminal && !dumb && !is_off(env("RUVYXA_FUN").as_deref()),
        // Not gated on `is_terminal`: a redirected log renders UTF-8 as well as
        // a terminal does, and the header emoji is already written
        // unconditionally. Only a terminal that cannot draw the glyphs — or a
        // user who says so — falls back to ASCII.
        unicode: !dumb && !is_on(env("RUVYXA_ASCII").as_deref()),
    }
}

/// A variable is "off" when it is set to an explicit negative. Following the
/// convention already used for adapter detection, an empty value counts as not
/// asking for the feature.
fn is_off(value: Option<&str>) -> bool {
    match value {
        None => false,
        Some(value) => {
            let value = value.trim();
            value.is_empty()
                || value == "0"
                || value.eq_ignore_ascii_case("false")
                || value.eq_ignore_ascii_case("off")
                || value.eq_ignore_ascii_case("no")
        }
    }
}

fn is_on(value: Option<&str>) -> bool {
    match value {
        None => false,
        Some(value) => !is_off(Some(value)),
    }
}

pub fn paint(value: impl AsRef<str>, code: &str) -> String {
    paint_when(capabilities().color, value, code)
}

/// The pure half of [`paint`]: colour decided by the caller.
pub fn paint_when(color: bool, value: impl AsRef<str>, code: &str) -> String {
    let value = value.as_ref();
    if !color {
        return value.to_string();
    }

    format!("\x1b[{code}m{value}\x1b[0m")
}

// ─── Roles ───────────────────────────────────────────────────────────────────
//
// A role says what a value *is*, and the palette decides what colour that
// becomes. Adding colour for decoration alone is what made every field cyan;
// the rule here is the one `styled_first_load` already follows — if two values
// carry different meaning, they get different colours, and if they carry the
// same meaning they get the same one everywhere.
//
// Every code stays inside the 16-colour range so a terminal without 256-colour
// support renders the same distinctions rather than approximating them.

pub fn heading(value: impl AsRef<str>) -> String {
    paint(value, "1;35")
}

/// The mascot and anything that carries the product's identity.
pub fn brand(value: impl AsRef<str>) -> String {
    paint(value, "1;33")
}

pub fn label(value: impl AsRef<str>) -> String {
    paint(value, "90")
}

/// A name, a word, a text value: the default for anything that is not a count,
/// a path, or a status.
pub fn accent(value: impl AsRef<str>) -> String {
    paint(value, "36")
}

/// A count or a measurement. Bright and bold so a number is findable in a
/// column of names — `doctor` prints twenty-five fields and the reader is
/// almost always looking for one of the eight numbers among them.
pub fn number(value: impl AsRef<str>) -> String {
    paint(value, "1;96")
}

/// Structural or descriptive information: a version, a target, a kind.
pub fn info(value: impl AsRef<str>) -> String {
    paint(value, "94")
}

/// A secondary classification that must stay distinguishable from [`info`] when
/// the two sit in the same column — page routes against API routes, for
/// instance.
pub fn note(value: impl AsRef<str>) -> String {
    paint(value, "95")
}

pub fn dim(value: impl AsRef<str>) -> String {
    paint(value, "90")
}

pub fn ok_text(value: impl AsRef<str>) -> String {
    paint(value, "32")
}

pub fn warn_text(value: impl AsRef<str>) -> String {
    paint(value, "33")
}

pub fn alert_text(value: impl AsRef<str>) -> String {
    paint(value, "31")
}

pub fn link(value: impl AsRef<str>) -> String {
    paint(value, "34")
}

pub fn success() -> String {
    ok_text("[ok]")
}

pub fn error_label() -> String {
    alert_text("[error]")
}
