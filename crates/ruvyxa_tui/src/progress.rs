//! Live progress: the runner track and the phase spinner.
//!
//! Both repaint the current line with a carriage return, so both are gated on
//! [`Capabilities::animate`] — a pipe, a log file, `TERM=dumb`, or `RUVYXA_FUN=0`
//! gets no repainting at all, and the phase line printed at the end is the only
//! record of the work. That is the same rule the old TTY-only progress bar
//! followed; it is stated once here instead of at each call site.
//!
//! The spinner ticks from its own thread while a phase blocks. That is safe
//! only because a phase body does not print: the build's own output is emitted
//! before the phase starts or after it finishes, and `tracing` diagnostics go to
//! stderr, which the spinner never touches. A future phase that prints to
//! stdout while running must take the spinner's line into account — use the
//! runner track instead, which is driven from the working thread.

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::layout::{PHASE_LABEL_WIDTH, format_duration, spaces};
use crate::mascot::{Glyphs, glyphs};
use crate::theme::{Capabilities, accent, capabilities, dim, label, ok_text};

/// Cells of track, excluding the runner itself. The runner is drawn between the
/// two halves, so the rendered width is constant as it advances.
pub const TRACK_WIDTH: usize = 26;

const SPINNER_INTERVAL: Duration = Duration::from_millis(90);

/// Advanced on every repaint so the dust behind the runner alternates even
/// while the underlying count is unchanged.
static TICK: AtomicUsize = AtomicUsize::new(0);

/// Redraws the runner track in place. Silent unless the terminal accepts
/// animation, so piped output and CI logs stay one line per event.
pub fn draw_progress_bar(enabled: bool, name: &str, done: usize, total: usize) {
    let capabilities = capabilities();
    if !enabled || total == 0 || !capabilities.animate {
        return;
    }

    let tick = TICK.fetch_add(1, Ordering::Relaxed);
    print!(
        "\r  {} {}{} {} {}/{} ",
        dim(glyphs(capabilities).pending),
        label(name),
        spaces(PHASE_LABEL_WIDTH, name.len()),
        runner_track(capabilities, done, total, tick),
        done,
        total
    );
    let _ = std::io::stdout().flush();
}

/// Clears a line drawn by [`draw_progress_bar`] so the phase line replaces it.
pub fn clear_progress_bar(enabled: bool) {
    if !enabled || !capabilities().animate {
        return;
    }
    clear_line();
}

fn clear_line() {
    print!("\r{}\r", " ".repeat(60 + TRACK_WIDTH));
    let _ = std::io::stdout().flush();
}

/// The coloured track: completed ground behind the runner, a puff of dust under
/// its feet, untouched ground ahead.
pub fn runner_track(capabilities: Capabilities, done: usize, total: usize, tick: usize) -> String {
    let glyphs = glyphs(capabilities);
    let (behind, runner, ahead) = runner_cells(glyphs, done, total, tick);
    format!("{}{}{}", accent(behind), runner, dim(ahead))
}

/// The unstyled halves of the track. Split out from [`runner_track`] because
/// the position and dust arithmetic is what can be wrong, and it is testable
/// only without escape codes in the way.
pub fn runner_cells(
    glyphs: Glyphs,
    done: usize,
    total: usize,
    tick: usize,
) -> (String, &'static str, String) {
    let filled = (TRACK_WIDTH * done.min(total))
        .checked_div(total)
        .unwrap_or(0);

    let mut behind = glyphs.filled.repeat(filled);
    if filled > 0 {
        // The cell the runner just left shows dust instead of solid ground.
        behind.truncate(behind.len() - glyphs.filled.len());
        behind.push_str(glyphs.dust[tick % glyphs.dust.len()]);
    }

    (
        behind,
        glyphs.runner,
        glyphs.empty.repeat(TRACK_WIDTH - filled),
    )
}

/// A phase line: what ran, what it produced, how long it took.
pub fn print_phase(name: &str, detail: String, duration: Duration) {
    println!("{}", phase_line(name, detail, duration));
}

pub fn phase_line(name: &str, detail: String, duration: Duration) -> String {
    format!(
        "  {} {}{} {} {}",
        ok_text(glyphs(capabilities()).done),
        label(name),
        spaces(PHASE_LABEL_WIDTH, name.len()),
        accent(detail),
        dim(format!("· {}", format_duration(duration)))
    )
}

/// A phase that has started but cannot report progress: the label spins with a
/// live elapsed counter until [`Spinner::finish`] replaces it with the result.
///
/// When the terminal does not accept animation nothing is drawn while the phase
/// runs, and `finish` prints the same phase line the non-animated build always
/// printed.
pub struct Spinner {
    started: Instant,
    name: String,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Spinner {
    pub fn start(name: &str) -> Self {
        let started = Instant::now();
        let stop = Arc::new(AtomicBool::new(false));
        let handle = if capabilities().animate {
            let stop = Arc::clone(&stop);
            let name = name.to_string();
            Some(std::thread::spawn(move || {
                let frames = glyphs(capabilities()).spinner;
                let mut frame = 0;
                while !stop.load(Ordering::Relaxed) {
                    print!(
                        "\r  {} {}{} {} ",
                        dim(frames[frame % frames.len()]),
                        label(&name),
                        spaces(PHASE_LABEL_WIDTH, name.len()),
                        dim(format_duration(started.elapsed()))
                    );
                    let _ = std::io::stdout().flush();
                    frame += 1;
                    std::thread::sleep(SPINNER_INTERVAL);
                }
            }))
        } else {
            None
        };

        Self {
            started,
            name: name.to_string(),
            stop,
            handle,
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// Stops the animation and prints the phase result, timed from `start`.
    pub fn finish(self, detail: String) {
        let elapsed = self.started.elapsed();
        self.finish_with(detail, elapsed);
    }

    /// Stops the animation and prints the phase result with a caller-supplied
    /// duration, for a phase that measures only part of its own span.
    pub fn finish_with(mut self, detail: String, duration: Duration) {
        self.stop_animation();
        print_phase(&self.name, detail, duration);
    }

    /// Stops the animation without printing, for a phase that turned out to
    /// have nothing to report.
    pub fn cancel(mut self) {
        self.stop_animation();
    }

    fn stop_animation(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
            clear_line();
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        // A phase that fails unwinds past `finish`; the thread must still stop,
        // or the error message is printed over a spinning line.
        self.stop_animation();
    }
}
