//! Bounded execution of synchronous child processes.
//!
//! Every place Ruvyxa shells out to a JavaScript runtime or a project-local
//! tool used to call [`std::process::Command::output`], which waits forever. A
//! child that keeps its event loop alive — an unawaited handle, a listening
//! socket, a stray `setInterval` in a config file or a plugin — therefore hung
//! the CLI with no output and no way out but killing it, and a `SIGKILL` or a
//! panic on the Rust side left the child orphaned because
//! [`std::process::Child`] does not terminate on drop.
//!
//! [`output_with_timeout`] closes both holes: the wait is bounded, and the
//! child is killed and reaped before the call returns on every path out.
//!
//! The async paths do not use this module — the worker pool and the plugin
//! middleware host have `tokio::time::timeout` and `kill_on_drop(true)` for the
//! same purpose. This is the synchronous half of that same rule.

use std::io::{self, Read};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// How often the wait loop re-checks a child that has not exited yet.
///
/// Short enough that a fast command is not noticeably delayed, long enough that
/// waiting out a slow type check does not spin a core.
const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Why a bounded child process did not produce output.
#[derive(Debug)]
pub enum ProcessError {
    /// The command could not be started, or its pipes could not be read.
    Io(io::Error),
    /// The child was still running when its budget ran out. It has been killed
    /// and reaped by the time this is returned.
    TimedOut {
        /// The budget that elapsed.
        after: Duration,
    },
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::TimedOut { after } => write!(
                formatter,
                "the process did not finish within {} seconds and was stopped",
                after.as_secs().max(1)
            ),
        }
    }
}

impl std::error::Error for ProcessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::TimedOut { .. } => None,
        }
    }
}

impl From<io::Error> for ProcessError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Run `command` to completion, or kill it once `timeout` elapses.
///
/// Behaves like [`Command::output`] on the happy path: stdout and stderr are
/// captured and the exit status is reported without interpretation, so a
/// command that fails is the caller's business, not this function's. Only the
/// unbounded wait is replaced.
///
/// stdin is closed rather than inherited. A child that blocks reading a console
/// that will never produce input is the same hang by another route, and no
/// caller here feeds a child interactively.
pub fn output_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> Result<Output, ProcessError> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Both pipes are drained on their own threads for the whole run. Reading
    // them after the wait instead would deadlock as soon as a child writes more
    // than one pipe buffer of output — which a failing `tsc` easily does — since
    // the child blocks on a full pipe and never reaches the exit this loop is
    // waiting for.
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();
    let stdout_reader = std::thread::spawn(move || drain(stdout_pipe.as_mut()));
    let stderr_reader = std::thread::spawn(move || drain(stderr_pipe.as_mut()));

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait()? {
            Some(status) => break Some(status),
            None if Instant::now() >= deadline => {
                // Kill and reap before returning: a caller that gives up must
                // not leave the process behind, which is exactly what made a
                // stalled build leave orphaned runtimes running.
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            None => std::thread::sleep(POLL_INTERVAL),
        }
    };

    // Joined after the child is gone either way, so the pipes are closed and
    // neither reader can still be blocked.
    let stdout = stdout_reader.join().unwrap_or_else(|_| Ok(Vec::new()))?;
    let stderr = stderr_reader.join().unwrap_or_else(|_| Ok(Vec::new()))?;

    let Some(status) = status else {
        return Err(ProcessError::TimedOut { after: timeout });
    };

    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn drain(pipe: Option<&mut impl Read>) -> io::Result<Vec<u8>> {
    let Some(pipe) = pipe else {
        return Ok(Vec::new());
    };
    let mut buffer = Vec::new();
    pipe.read_to_end(&mut buffer)?;
    Ok(buffer)
}

/// Budget for a `--version`-style probe used to detect an installed tool.
///
/// A runtime that does not answer this quickly is not usable as a runtime, and
/// `doctor` reporting "missing" beats `doctor` never returning.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Budget for loading a project's `ruvyxa.config.ts` in a JavaScript runtime.
///
/// Generous because config files legitimately import application modules, but
/// bounded because a config that opens a server or a database handle keeps the
/// runtime alive after it has already produced its answer.
pub const CONFIG_LOAD_TIMEOUT: Duration = Duration::from_secs(60);

/// Budget for one adapter build hook.
///
/// Adapter hooks copy build output and can legitimately run for minutes on a
/// large site.
pub const ADAPTER_HOOK_TIMEOUT: Duration = Duration::from_secs(600);

/// Budget for a project type check (`tsc --noEmit`).
///
/// The slowest tool Ruvyxa invokes, and the one most likely to be slow for
/// legitimate reasons, so the bound only catches a genuinely wedged process.
pub const TYPECHECK_TIMEOUT: Duration = Duration::from_secs(900);

/// Budget for a CSS toolchain invocation such as the Tailwind CLI.
pub const STYLE_TOOL_TIMEOUT: Duration = Duration::from_secs(300);

/// Budget for rendering one page or API route in a one-shot runtime process.
///
/// This is a request being served, so the bound is what a developer will wait
/// before deciding the page is broken.
pub const RENDER_TIMEOUT: Duration = Duration::from_secs(120);

#[cfg(test)]
mod tests {
    use super::*;

    fn node_available() -> bool {
        crate::JavaScriptRuntime::Node.is_available()
    }

    #[test]
    fn returns_output_and_status_for_a_command_that_finishes() {
        if !node_available() {
            eprintln!("skipping: node is not available on this machine");
            return;
        }
        let mut command = Command::new(crate::JavaScriptRuntime::Node.executable());
        command.arg("-e").arg("console.log('ok'); process.exit(2)");

        let output = output_with_timeout(&mut command, Duration::from_secs(30)).unwrap();
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ok");
        assert_eq!(output.status.code(), Some(2));
    }

    /// The hang this module exists to prevent: a child that has answered but
    /// keeps its event loop alive.
    #[test]
    fn kills_a_child_that_outlives_its_budget() {
        if !node_available() {
            eprintln!("skipping: node is not available on this machine");
            return;
        }
        let mut command = Command::new(crate::JavaScriptRuntime::Node.executable());
        command
            .arg("-e")
            .arg("console.log('answered'); setInterval(() => {}, 1000)");

        let started = Instant::now();
        let error = output_with_timeout(&mut command, Duration::from_millis(400))
            .expect_err("a child holding the event loop open must not resolve");

        assert!(
            matches!(error, ProcessError::TimedOut { .. }),
            "expected a timeout, got {error}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the wait must end on its own budget"
        );
    }

    /// A child that writes more than one pipe buffer must not deadlock the wait.
    #[test]
    fn captures_output_larger_than_a_pipe_buffer() {
        if !node_available() {
            eprintln!("skipping: node is not available on this machine");
            return;
        }
        let mut command = Command::new(crate::JavaScriptRuntime::Node.executable());
        command
            .arg("-e")
            .arg("process.stdout.write('x'.repeat(2_000_000))");

        let output = output_with_timeout(&mut command, Duration::from_secs(60)).unwrap();
        assert_eq!(output.stdout.len(), 2_000_000);
        assert!(output.status.success());
    }

    /// stdin is closed, so a child that reads it gets EOF instead of blocking.
    #[test]
    fn closes_stdin_so_a_reading_child_is_not_left_waiting() {
        if !node_available() {
            eprintln!("skipping: node is not available on this machine");
            return;
        }
        let mut command = Command::new(crate::JavaScriptRuntime::Node.executable());
        command.arg("-e").arg(
            "let seen = ''; process.stdin.on('data', (chunk) => { seen += chunk }); \
             process.stdin.on('end', () => { console.log(`eof:${seen.length}`) })",
        );

        let output = output_with_timeout(&mut command, Duration::from_secs(30)).unwrap();
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "eof:0");
    }
}
