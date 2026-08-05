//! The Ruvyxa fox: the one place that decides how the mascot is drawn.
//!
//! The fox already existed in the command headers as a static emoji. It moves
//! now — it runs the length of a progress track, the same character the demo's
//! `ruvyxa-runner` mini-game puts on screen — but only where movement is safe:
//! a real terminal that has not opted out of animation.
//!
//! [`tui_header_title`] deliberately does *not* consult terminal capabilities.
//! The header emoji is part of the product name in every transcript, including
//! piped output, and a test pins that spelling.

use crate::theme::Capabilities;

/// Glyphs for one drawing style. Two sets exist so a terminal without box
/// drawing still gets a readable track rather than replacement characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyphs {
    pub filled: &'static str,
    pub empty: &'static str,
    /// Two frames of dust kicked up behind the runner. Alternating these is
    /// what makes the fox look like it is running rather than sliding.
    pub dust: [&'static str; 2],
    pub runner: &'static str,
    pub spinner: &'static [&'static str],
    pub done: &'static str,
    pub pending: &'static str,
}

pub const UNICODE_GLYPHS: Glyphs = Glyphs {
    filled: "▰",
    empty: "▱",
    dust: ["·", "˙"],
    runner: "🦊",
    spinner: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
    done: "✓",
    pending: "◌",
};

pub const ASCII_GLYPHS: Glyphs = Glyphs {
    filled: "#",
    empty: "-",
    dust: [".", ","],
    runner: ">",
    spinner: &["|", "/", "-", "\\"],
    done: "+",
    pending: "o",
};

pub fn glyphs(capabilities: Capabilities) -> Glyphs {
    if capabilities.unicode {
        UNICODE_GLYPHS
    } else {
        ASCII_GLYPHS
    }
}

/// The title used by every command header. Stable across terminals by design.
pub fn tui_header_title(title: impl AsRef<str>) -> String {
    format!("🦊 Ruvyxa {}", title.as_ref())
}
