//! Terminal presentation for every Ruvyxa binary.
//!
//! Colour, field layout, tables, progress, and the mascot lived in two places
//! before this crate existed — `ruvyxa_cli::ui` and
//! `ruvyxa_dev_server::cli_output` — with the same ANSI codes, the same
//! capability checks, and two different label widths, which is why `ruvyxa dev`
//! and `ruvyxa build` printed their values in different columns. Both now
//! re-export from here, so a change to how Ruvyxa looks is a change in one
//! file.
//!
//! This crate is a leaf: it depends on nothing in the workspace, and nothing
//! here knows what a route, a bundle, or a request is. It decides how output
//! looks and never what output means.

pub mod layout;
pub mod mascot;
pub mod progress;
pub mod theme;

pub use layout::*;
pub use mascot::*;
pub use progress::*;
pub use theme::*;

#[cfg(test)]
mod tests;
