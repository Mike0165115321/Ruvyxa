//! Toolchain and dependency probing for `ruvyxa doctor`.
//!
//! Reads the project's package manager, installed tool versions, and React
//! compatibility off the filesystem. Every probe degrades to a printable status
//! rather than failing: for `doctor`, a missing tool is a finding to report, not
//! an error to abort on.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use anyhow::Context;
use ruvyxa_dev_server::JavaScriptRuntime;

pub(crate) fn detect_package_manager(root: &Path) -> String {
    if find_upwards(root, "pnpm-lock.yaml").is_some() {
        "pnpm".to_string()
    } else if find_upwards(root, "package-lock.json").is_some() {
        "npm".to_string()
    } else if find_upwards(root, "yarn.lock").is_some() {
        "yarn".to_string()
    } else if find_upwards(root, "bun.lock").is_some() || find_upwards(root, "bun.lockb").is_some()
    {
        "bun".to_string()
    } else {
        "unknown".to_string()
    }
}

pub(crate) fn find_upwards(root: &Path, file_name: &str) -> Option<PathBuf> {
    let mut current = ruvyxa_diagnostics::normalized_canonical_path(root);

    loop {
        let candidate = current.join(file_name);
        if candidate.exists() {
            return Some(candidate);
        }

        if !current.pop() {
            return None;
        }
    }
}

pub(crate) fn tool_version(command: &str, args: &[&str]) -> String {
    // Bounded: `doctor` probes several tools in a row, and one that never
    // answers would stall the whole report instead of being listed as missing.
    let mut probe = ProcessCommand::new(command);
    probe.args(args);
    match ruvyxa_dev_server::process::output_with_timeout(
        &mut probe,
        ruvyxa_dev_server::process::PROBE_TIMEOUT,
    ) {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "missing".to_string(),
    }
}

/// Reports Bun's version using the same executable resolution as the build
/// and dev-server runtimes. Windows exposes Bun as a `bun.cmd` shim, which a
/// plain `Command::new("bun")` cannot launch, so a naive check reports "missing"
/// even when `bun --version` succeeds in a shell.
pub(crate) fn bun_version() -> String {
    let mut probe = ProcessCommand::new(JavaScriptRuntime::Bun.executable());
    probe.arg("--version");
    match ruvyxa_dev_server::process::output_with_timeout(
        &mut probe,
        ruvyxa_dev_server::process::PROBE_TIMEOUT,
    ) {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "missing".to_string(),
    }
}

pub(crate) fn local_binary_upwards(root: &Path, binary: &str) -> Option<PathBuf> {
    let binary = if cfg!(windows) {
        format!("{binary}.cmd")
    } else {
        binary.to_string()
    };
    let mut current = ruvyxa_diagnostics::normalized_canonical_path(root);

    loop {
        let candidate = current.join("node_modules").join(".bin").join(&binary);
        if candidate.is_file() {
            return Some(candidate);
        }

        if !current.pop() {
            return None;
        }
    }
}

pub(crate) fn read_package_json(path: &Path) -> anyhow::Result<serde_json::Value> {
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&source).with_context(|| format!("failed to parse {}", path.display()))
}

pub(crate) fn dependency_version(package: &serde_json::Value, name: &str) -> Option<String> {
    ["dependencies", "devDependencies", "peerDependencies"]
        .into_iter()
        .find_map(|section| {
            package
                .get(section)
                .and_then(|deps| deps.get(name))
                .and_then(|version| version.as_str())
                .map(str::to_string)
        })
}

/// Every Ruvyxa package the project depends on, sorted by name.
///
/// A project pulls in `ruvyxa` plus any number of `@ruvyxa/*` packages, and
/// each is versioned independently, so listing them is the only way to see the
/// set a project is actually running.
pub(crate) fn ruvyxa_dependencies(package: &serde_json::Value) -> Vec<(String, String)> {
    let mut found = BTreeMap::<String, String>::new();

    for section in ["dependencies", "devDependencies", "peerDependencies"] {
        let Some(deps) = package.get(section).and_then(|value| value.as_object()) else {
            continue;
        };

        for (name, version) in deps {
            if name == "ruvyxa" || name.starts_with("@ruvyxa/") {
                found.insert(
                    name.clone(),
                    version.as_str().unwrap_or("unknown").to_string(),
                );
            }
        }
    }

    found.into_iter().collect()
}

/// Compare the npm `ruvyxa` dependency against the CLI binary running the check.
///
/// The native CLI and the npm package are released together and read each
/// other's contracts — a manifest written by one version and served by another
/// is a class of failure that only appears at runtime, so the drift is worth
/// naming here rather than leaving it to be discovered in production.
pub(crate) fn cli_version_match(package_version: Option<&str>, cli_version: &str) -> String {
    let Some(package_version) = package_version else {
        return "missing".to_string();
    };

    // A workspace or link protocol resolves to the checkout itself, so there is
    // no published version to compare and nothing to warn about. Reporting
    // these as drift is what made the framework's own repository fail its own
    // doctor.
    if package_version.starts_with("workspace:")
        || package_version.starts_with("link:")
        || package_version.starts_with("file:")
    {
        return format!("ok ({package_version})");
    }

    let declared = package_version.trim_start_matches(['^', '~', '=', 'v', ' ']);
    if declared == "*" || declared.is_empty() || declared.eq_ignore_ascii_case("latest") {
        return format!("ok (unpinned, cli {cli_version})");
    }
    if declared == cli_version {
        return format!("ok ({cli_version})");
    }

    match (major_version(declared), major_version(cli_version)) {
        (Some(left), Some(right)) if left == right => {
            format!("ok (package {declared}, cli {cli_version})")
        }
        _ => format!("drift: package {declared}, cli {cli_version}"),
    }
}

pub(crate) fn react_compatibility(package: &serde_json::Value) -> String {
    let Some(react) = dependency_version(package, "react") else {
        return "missing react".to_string();
    };
    let Some(react_dom) = dependency_version(package, "react-dom") else {
        return "missing react-dom".to_string();
    };

    match (major_version(&react), major_version(&react_dom)) {
        (Some(left), Some(right)) if left == right => format!("ok (major {left})"),
        (Some(left), Some(right)) => format!("mismatch react {left} vs react-dom {right}"),
        _ => "unknown version format".to_string(),
    }
}

pub(crate) fn major_version(version: &str) -> Option<u64> {
    let digits = version
        .trim_start_matches(|character: char| !character.is_ascii_digit())
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}

pub(crate) fn duplicate_dependencies(package: &serde_json::Value) -> Vec<String> {
    let mut seen = BTreeMap::<String, String>::new();
    let mut duplicates = Vec::new();

    for section in ["dependencies", "devDependencies", "peerDependencies"] {
        let Some(deps) = package.get(section).and_then(|value| value.as_object()) else {
            continue;
        };

        for (name, version) in deps {
            let version = version.as_str().unwrap_or("unknown").to_string();
            if let Some(previous) = seen.insert(name.clone(), version.clone())
                && previous != version
            {
                duplicates.push(format!("{name} ({previous}, {version})"));
            }
        }
    }

    duplicates.sort();
    duplicates
}
