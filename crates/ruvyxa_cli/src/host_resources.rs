//! How much of this machine a build is allowed to use.
//!
//! Build concurrency used to be sized from the core count alone. That reads as
//! "use the machine", but memory, not cores, is what runs out: every extra
//! concurrent route bundle holds its own parser arenas and module graph, and
//! every extra prerender worker is a whole JavaScript runtime process. On a
//! 16-core developer machine that cost about 100MB over serial bundling for a
//! 1.4x speedup; on a 64-core CI runner with a small memory limit the same rule
//! asks for far more memory in exchange for almost no additional speed, and the
//! build is killed rather than slowed.
//!
//! So concurrency is sized by whichever of cores and free memory runs out
//! first. When free memory cannot be determined the core count is used
//! unchanged, which is exactly the previous behaviour — an unknown budget must
//! not silently serialize a build.

/// Memory one concurrent route bundle is assumed to need.
///
/// Measured on `examples/demo`: peak resident memory rose from about 120MB to
/// about 218MB going from one worker to sixteen, so roughly 6.5MB per worker.
/// Rounded up, because a route larger than the demo's costs more and being
/// wrong in this direction only costs parallelism.
const BUNDLE_WORKER_MEMORY_BYTES: u64 = 12 * 1024 * 1024;

/// Memory one prerender worker is assumed to need.
///
/// A whole JavaScript runtime process with the route's module graph loaded,
/// which is an order of magnitude more than a bundling worker.
const PRERENDER_WORKER_MEMORY_BYTES: u64 = 96 * 1024 * 1024;

/// Memory reserved for everything that is not a worker.
///
/// The bundler's shared caches, the route manifest, image decoding, and the
/// operating system. Subtracted before any worker budget is computed so a
/// nearly-full machine is told to run one worker rather than a negative number.
const BASELINE_MEMORY_BYTES: u64 = 256 * 1024 * 1024;

/// Free physical memory in bytes, or `None` when it cannot be determined.
///
/// Deliberately "available" rather than "total": a build shares the machine
/// with an editor, a browser, and often a language server, and sizing against
/// total memory is how a build starts swapping.
#[must_use]
pub(crate) fn available_memory_bytes() -> Option<u64> {
    platform::available_memory_bytes()
}

/// The largest number of workers `available` memory can hold, given a per-worker
/// cost, or `None` when memory is unknown.
fn memory_worker_budget(per_worker_bytes: u64) -> Option<usize> {
    let available = available_memory_bytes()?;
    let usable = available.saturating_sub(BASELINE_MEMORY_BYTES);
    // Always at least one: refusing to run because memory is tight would turn a
    // slow build into no build.
    Some(
        usize::try_from(usable / per_worker_bytes)
            .unwrap_or(usize::MAX)
            .max(1),
    )
}

/// Concurrency for route bundling, bounded by cores and by free memory.
#[must_use]
pub(crate) fn bundle_worker_budget(cpu_budget: usize) -> usize {
    match memory_worker_budget(BUNDLE_WORKER_MEMORY_BYTES) {
        Some(memory_budget) => cpu_budget.min(memory_budget).max(1),
        None => cpu_budget.max(1),
    }
}

/// Concurrency for prerendering, bounded by cores and by free memory.
#[must_use]
pub(crate) fn prerender_worker_budget(cpu_budget: usize) -> usize {
    match memory_worker_budget(PRERENDER_WORKER_MEMORY_BYTES) {
        Some(memory_budget) => cpu_budget.min(memory_budget).max(1),
        None => cpu_budget.max(1),
    }
}

#[cfg(target_os = "linux")]
mod platform {
    /// Read `MemAvailable` from `/proc/meminfo`.
    ///
    /// `MemAvailable` rather than `MemFree`: the kernel's own estimate of what a
    /// new workload can claim without swapping, which counts reclaimable page
    /// cache that `MemFree` reports as used.
    pub(super) fn available_memory_bytes() -> Option<u64> {
        let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
        let kibibytes = parse_mem_available_kib(&meminfo)?;
        Some(kibibytes.saturating_mul(1024))
    }

    fn parse_mem_available_kib(meminfo: &str) -> Option<u64> {
        meminfo
            .lines()
            .find_map(|line| line.strip_prefix("MemAvailable:"))
            .and_then(|value| value.split_whitespace().next())
            .and_then(|value| value.parse::<u64>().ok())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn reads_mem_available_from_the_kernel_format() {
            let meminfo = "MemTotal:       16316108 kB\nMemFree:         1000 kB\n\
                           MemAvailable:    8158054 kB\nBuffers:          123 kB\n";
            assert_eq!(parse_mem_available_kib(meminfo), Some(8_158_054));
        }

        /// Containers and older kernels can omit the field; an absent value must
        /// read as "unknown", never as zero.
        #[test]
        fn reports_unknown_when_mem_available_is_absent() {
            assert_eq!(parse_mem_available_kib("MemTotal: 16316108 kB\n"), None);
        }
    }
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    pub(super) fn available_memory_bytes() -> Option<u64> {
        let mut status = MEMORYSTATUSEX {
            dwLength: u32::try_from(size_of::<MEMORYSTATUSEX>()).ok()?,
            ..unsafe { std::mem::zeroed() }
        };
        // SAFETY: `status` is a correctly sized, zero-initialised
        // MEMORYSTATUSEX with `dwLength` set, which is the entire
        // contract of GlobalMemoryStatusEx.
        let ok = unsafe { GlobalMemoryStatusEx(&raw mut status) };
        if ok == 0 {
            return None;
        }
        Some(status.ullAvailPhys)
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod platform {
    /// Total physical memory via `sysctl hw.memsize`.
    ///
    /// Darwin has no cheap equivalent of `MemAvailable`, and the page-level
    /// accounting that would approximate it needs the Mach VM statistics API.
    /// Total memory halved is used instead: a deliberate underestimate, so the
    /// budget errs toward fewer workers on a busy machine.
    pub(super) fn available_memory_bytes() -> Option<u64> {
        let mut memsize: u64 = 0;
        let mut length = size_of::<u64>();
        let name = c"hw.memsize";
        // SAFETY: `name` is a NUL-terminated C string naming a sysctl that
        // returns a u64, and `memsize`/`length` describe a matching buffer.
        let status = unsafe {
            libc::sysctlbyname(
                name.as_ptr(),
                (&raw mut memsize).cast(),
                &raw mut length,
                std::ptr::null_mut(),
                0,
            )
        };
        if status != 0 || memsize == 0 {
            return None;
        }
        Some(memsize / 2)
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos", target_os = "ios")))]
mod platform {
    /// Unknown on this platform, so concurrency stays sized by cores alone.
    pub(super) fn available_memory_bytes() -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A budget is never zero, whatever the machine reports: a build that runs
    /// slowly is a build, and one with no workers is a hang.
    #[test]
    fn a_budget_is_always_at_least_one_worker() {
        assert!(bundle_worker_budget(0) >= 1);
        assert!(prerender_worker_budget(0) >= 1);
        assert!(bundle_worker_budget(64) >= 1);
    }

    /// Memory can only lower the request, never raise it. Asking for one worker
    /// must not produce eight because the machine is large.
    #[test]
    fn memory_never_raises_the_requested_concurrency() {
        for requested in [1_usize, 2, 4, 16, 64] {
            assert!(bundle_worker_budget(requested) <= requested.max(1));
            assert!(prerender_worker_budget(requested) <= requested.max(1));
        }
    }

    /// A prerender worker is a whole runtime process, so it must never be
    /// granted more concurrency than a bundling worker at the same request.
    #[test]
    fn prerender_workers_are_budgeted_at_least_as_tightly_as_bundle_workers() {
        assert!(prerender_worker_budget(64) <= bundle_worker_budget(64));
    }

    /// The probe must either answer with a plausible figure or admit it cannot.
    /// A zero would silently clamp every build to a single worker.
    #[test]
    fn the_memory_probe_reports_a_usable_figure_or_nothing() {
        if let Some(available) = available_memory_bytes() {
            assert!(
                available >= 16 * 1024 * 1024,
                "a machine running this test has more than 16MB free, got {available}"
            );
        }
    }
}
