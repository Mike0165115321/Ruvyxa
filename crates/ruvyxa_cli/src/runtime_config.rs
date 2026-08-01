//! Turning CLI arguments plus `ruvyxa.config.*` into a runnable configuration.
//!
//! Two server configurations are produced here — `dev` and `start` — and they
//! resolve the same settings from the same two sources, with an explicit flag
//! always beating the config file. Keeping both in one module is deliberate: a
//! setting added to one and forgotten in the other is the failure mode, and
//! here the omission is visible.
//!
//! This module also owns adapter inspection and the JavaScript runtime choice
//! (Node or Bun), including the process-wide override a `--runtime` flag sets
//! before any command runs.

use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::Duration;

use anyhow::Context;
use clap::ValueEnum;
use ruvyxa_dev_server::{JavaScriptRuntime, ServerConfig, find_runtime_script};

use crate::*;

pub(crate) fn dev_server_config(
    args: &ServerArgs,
    config: &ProjectConfig,
) -> anyhow::Result<ServerConfig> {
    let mut server = ServerConfig::dev(
        &args.root,
        args.host
            .clone()
            .or_else(|| config.server.host.clone())
            .unwrap_or_else(|| "localhost".to_string()),
        args.port.or(config.server.port).unwrap_or(3000),
    );
    let out_dir = args.root.join(config.out_dir());
    server.app_dir = args.root.join(config.app_dir());
    server.public_dir = args.root.join("public");
    server.client_dir = out_dir.join("client");
    server.prerender_dir = out_dir.join("prerender");
    server.cache_route_manifest = config.cache.route_manifest.unwrap_or(true);
    server.cache_css = config.cache.css.unwrap_or(true);
    server.style_entries = config.style_entries(&args.root);
    server.prebundle_dependencies = config.build.prebundle_dependencies.unwrap_or(true);
    server.runtime = config.javascript_runtime();
    server.jsx_runtime = parse_jsx_runtime(config.build.jsx_runtime.as_deref())?;
    server.error_overlay = config.debug.overlay.unwrap_or(true);
    server.debug_traces = config.debug.traces.unwrap_or(false);
    server.action_body_limit_bytes = config
        .security
        .action_body_limit_bytes
        .unwrap_or(server.action_body_limit_bytes);
    server.api_body_limit_bytes = config
        .security
        .api_body_limit_bytes
        .unwrap_or(server.api_body_limit_bytes);
    server.plugin_response_body_limit_bytes = config
        .security
        .plugin_response_body_limit_bytes
        .unwrap_or(server.plugin_response_body_limit_bytes);
    if let Some(rate_limit) = &config.security.action_rate_limit {
        server.action_rate_limit_max = rate_limit.max.unwrap_or(server.action_rate_limit_max);
        server.action_rate_limit_window = Duration::from_secs(
            rate_limit
                .window
                .unwrap_or(server.action_rate_limit_window.as_secs()),
        );
    }
    server.same_origin_actions = config
        .security
        .same_origin_actions
        .unwrap_or(server.same_origin_actions);
    server.fetch_metadata_actions = config
        .security
        .fetch_metadata_actions
        .unwrap_or(server.fetch_metadata_actions);
    server.trusted_proxies = parse_trusted_proxies(&config.security.trusted_proxy_ips)?;
    server.security_headers = config
        .security
        .security_headers
        .unwrap_or(server.security_headers);
    server.middleware = config.middleware.clone();
    server.plugins_enabled = !config.plugins.is_empty();
    server.plugin_head = collect_plugin_head(&config.plugins);
    server.default_render_strategy = config.rendering.default_strategy;
    server.default_revalidate = config.rendering.default_revalidate;
    Ok(server)
}

pub(crate) fn production_server_config(
    args: &ServerArgs,
    config: &ProjectConfig,
) -> anyhow::Result<ServerConfig> {
    let mut server = ServerConfig::production(
        &args.root,
        args.host
            .clone()
            .or_else(|| config.server.host.clone())
            .unwrap_or_else(|| "localhost".to_string()),
        args.port.or(config.server.port).unwrap_or(3000),
    );
    let out_dir = args.root.join(config.out_dir());
    server.app_dir = out_dir.join("server").join(config.app_dir());
    server.public_dir = out_dir.join("assets");
    server.client_dir = out_dir.join("client");
    server.prerender_dir = out_dir.join("prerender");
    server.cache_route_manifest = config.cache.route_manifest.unwrap_or(true);
    server.cache_css = config.cache.css.unwrap_or(true);
    server.style_entries = config.style_entries(&out_dir.join("server"));
    server.runtime = config.javascript_runtime();
    server.jsx_runtime = parse_jsx_runtime(config.build.jsx_runtime.as_deref())?;
    server.action_body_limit_bytes = config
        .security
        .action_body_limit_bytes
        .unwrap_or(server.action_body_limit_bytes);
    server.api_body_limit_bytes = config
        .security
        .api_body_limit_bytes
        .unwrap_or(server.api_body_limit_bytes);
    server.plugin_response_body_limit_bytes = config
        .security
        .plugin_response_body_limit_bytes
        .unwrap_or(server.plugin_response_body_limit_bytes);
    if let Some(rate_limit) = &config.security.action_rate_limit {
        server.action_rate_limit_max = rate_limit.max.unwrap_or(server.action_rate_limit_max);
        server.action_rate_limit_window = Duration::from_secs(
            rate_limit
                .window
                .unwrap_or(server.action_rate_limit_window.as_secs()),
        );
    }
    server.same_origin_actions = config
        .security
        .same_origin_actions
        .unwrap_or(server.same_origin_actions);
    server.fetch_metadata_actions = config
        .security
        .fetch_metadata_actions
        .unwrap_or(server.fetch_metadata_actions);
    server.trusted_proxies = parse_trusted_proxies(&config.security.trusted_proxy_ips)?;
    server.security_headers = config
        .security
        .security_headers
        .unwrap_or(server.security_headers);
    server.middleware = config.middleware.clone();
    server.plugins_enabled = !config.plugins.is_empty();
    server.plugin_head = collect_plugin_head(&config.plugins);
    server.default_render_strategy = config.rendering.default_strategy;
    server.default_revalidate = config.rendering.default_revalidate;
    Ok(server)
}

pub(crate) fn load_project_config(root: &Path) -> anyhow::Result<ProjectConfig> {
    let runtime_override = runtime_override()?;
    let bootstrap_runtime = runtime_override.unwrap_or_else(default_javascript_runtime);
    let Some(renderer) = find_runtime_script(root, "config-renderer.mjs") else {
        let mut config = ProjectConfig {
            config_dependency_hash: "no-config".to_string(),
            ..ProjectConfig::default()
        };
        config.javascript_runtime_override = Some(bootstrap_runtime);
        config.validate_paths()?;
        return Ok(config);
    };

    let mut result = run_config_renderer(root, &renderer, bootstrap_runtime)?;
    if !result.ok {
        anyhow::bail!(
            "config load failed: {} {}",
            result.code.unwrap_or_else(|| "RUV1600".to_string()),
            result
                .message
                .or(result.stack)
                .unwrap_or_else(|| "unknown config error".to_string())
        )
    }

    let mut config = result.config.take().unwrap_or_default();
    let selected_runtime = runtime_override.unwrap_or_else(|| config.javascript_runtime());
    if selected_runtime != bootstrap_runtime {
        result = run_config_renderer(root, &renderer, selected_runtime)?;
        if !result.ok {
            anyhow::bail!(
                "config load failed: {} {}",
                result.code.unwrap_or_else(|| "RUV1600".to_string()),
                result
                    .message
                    .or(result.stack)
                    .unwrap_or_else(|| "unknown config error".to_string())
            )
        }
        config = result.config.take().unwrap_or_default();
    }
    config.javascript_runtime_override = runtime_override;
    let dependency_hash = required_config_dependency_hash(&result)?;
    config.config_dependency_hash = dependency_hash;
    config.validate_paths()?;
    Ok(config)
}

pub(crate) fn run_config_renderer(
    root: &Path,
    renderer: &Path,
    runtime: JavaScriptRuntime,
) -> anyhow::Result<ConfigRendererOutput> {
    let output = ProcessCommand::new(runtime.executable())
        .arg(renderer)
        .arg(root)
        .env("RUVYXA_RUNTIME", runtime.command())
        .output()
        .with_context(|| {
            format!(
                "failed to load config with {} for {}",
                runtime.command(),
                root.display()
            )
        })?;
    parse_config_renderer_output(
        root,
        &output.stdout,
        &output.stderr,
        &output.status.to_string(),
    )
}

pub(crate) fn run_adapter_runner(
    root: &Path,
    staging_dir: &Path,
    runtime: JavaScriptRuntime,
    adapter_name: Option<&str>,
) -> anyhow::Result<Vec<AdapterArtifactReport>> {
    let runner = find_runtime_script(root, "adapter-runner.mjs").ok_or_else(|| {
        anyhow::anyhow!(
            "adapter build hook requires runtime/adapter-runner.mjs; reinstall the ruvyxa package"
        )
    })?;
    let mut command = ProcessCommand::new(runtime.executable());
    command
        .arg(runner)
        .arg(root)
        .arg(staging_dir)
        .env("RUVYXA_RUNTIME", runtime.command());
    if let Some(adapter_name) = adapter_name {
        command.arg(adapter_name);
    }
    let output = command.output().with_context(|| {
        format!(
            "failed to run adapter build hook with {} for {}",
            runtime.command(),
            root.display()
        )
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let result: AdapterRunnerOutput = serde_json::from_str(&stdout).with_context(|| {
        format!(
            "adapter runner returned invalid output for {}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            root.display(),
            output.status,
            diagnostic_stream(&stdout),
            diagnostic_stream(&stderr),
        )
    })?;
    if !result.ok {
        anyhow::bail!(
            "adapter build hook failed: {} {}",
            result.code.unwrap_or_else(|| "RUV2200".to_string()),
            result
                .message
                .or(result.stack)
                .unwrap_or_else(|| "unknown adapter error".to_string())
        );
    }
    result
        .result
        .map(serde_json::from_value)
        .transpose()
        .context("adapter runner returned an invalid artifact report")
        .map(Option::unwrap_or_default)
}

pub(crate) fn inspect_adapter(
    root: &Path,
    out_dir: &Path,
    runtime: JavaScriptRuntime,
    adapter_name: Option<&str>,
) -> anyhow::Result<Option<AdapterInspection>> {
    let runner = find_runtime_script(root, "adapter-runner.mjs").ok_or_else(|| {
        anyhow::anyhow!(
            "adapter inspection requires runtime/adapter-runner.mjs; reinstall the ruvyxa package"
        )
    })?;
    let mut command = ProcessCommand::new(runtime.executable());
    command
        .arg(runner)
        .arg(root)
        .arg(out_dir)
        .env("RUVYXA_RUNTIME", runtime.command())
        .env("RUVYXA_ADAPTER_RUNNER_MODE", "inspect");
    if let Some(adapter_name) = adapter_name {
        command.arg(adapter_name);
    }
    let output = command.output().with_context(|| {
        format!(
            "failed to inspect adapter with {} for {}",
            runtime.command(),
            root.display()
        )
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let result: AdapterRunnerOutput = serde_json::from_str(&stdout).with_context(|| {
        format!(
            "adapter inspector returned invalid output for {}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            root.display(),
            output.status,
            diagnostic_stream(&stdout),
            diagnostic_stream(&stderr),
        )
    })?;
    if !result.ok {
        anyhow::bail!(
            "adapter inspection failed: {} {}",
            result.code.unwrap_or_else(|| "RUV2200".to_string()),
            result
                .message
                .or(result.stack)
                .unwrap_or_else(|| "unknown adapter error".to_string())
        );
    }
    result
        .result
        .map(serde_json::from_value)
        .transpose()
        .context("adapter inspector returned an invalid capability report")
}

/// Process-wide runtime override set by the `--runtime` CLI flag. Takes
/// precedence over `RUVYXA_RUNTIME` and `config.runtime`.
pub(crate) static CLI_RUNTIME_OVERRIDE: std::sync::OnceLock<JavaScriptRuntime> =
    std::sync::OnceLock::new();

pub(crate) fn command_runtime(command: &Command) -> Option<CliRuntime> {
    match command {
        Command::Dev(args) | Command::Start(args) | Command::Preview(args) => args.runtime,
        Command::Build(args) => args.runtime,
        Command::Check(args)
        | Command::Routes(args)
        | Command::Clean(args)
        | Command::TestParity(args) => args.runtime,
        Command::Analyze(args) => args.runtime,
        Command::Doctor(args) => args.runtime,
        Command::Trace(_) | Command::Bench(_) | Command::Plugin(_) => None,
    }
}

pub(crate) fn set_cli_runtime_override(runtime: Option<CliRuntime>) {
    if let Some(runtime) = runtime {
        let _ = CLI_RUNTIME_OVERRIDE.set(runtime.into());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum CliRuntime {
    Node,
    Bun,
}

impl From<CliRuntime> for JavaScriptRuntime {
    fn from(value: CliRuntime) -> Self {
        match value {
            CliRuntime::Node => Self::Node,
            CliRuntime::Bun => Self::Bun,
        }
    }
}

pub(crate) fn runtime_override() -> anyhow::Result<Option<JavaScriptRuntime>> {
    if let Some(runtime) = CLI_RUNTIME_OVERRIDE.get() {
        return Ok(Some(*runtime));
    }
    let Ok(value) = std::env::var("RUVYXA_RUNTIME") else {
        return Ok(None);
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "node" => Ok(Some(JavaScriptRuntime::Node)),
        "bun" => Ok(Some(JavaScriptRuntime::Bun)),
        _ => anyhow::bail!("RUVYXA_RUNTIME must be either 'node' or 'bun'"),
    }
}

pub(crate) fn default_javascript_runtime() -> JavaScriptRuntime {
    JavaScriptRuntime::detect()
}

pub(crate) fn required_config_dependency_hash(
    result: &ConfigRendererOutput,
) -> anyhow::Result<String> {
    result
        .dependency_hash
        .as_ref()
        .filter(|hash| !hash.is_empty())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("config renderer returned success without dependencyHash"))
}

pub(crate) fn parse_config_renderer_output(
    root: &Path,
    stdout: &[u8],
    stderr: &[u8],
    status: &str,
) -> anyhow::Result<ConfigRendererOutput> {
    let stdout = String::from_utf8_lossy(stdout);
    let stderr = String::from_utf8_lossy(stderr);
    serde_json::from_str(&stdout).with_context(|| {
        format!(
            "config renderer returned invalid output for {}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            root.display(),
            status,
            diagnostic_stream(&stdout),
            diagnostic_stream(&stderr),
        )
    })
}

pub(crate) fn build_cache_dir(root: &Path, cache: &CacheConfigOptions) -> PathBuf {
    resolve_build_cache_dir(
        root,
        cache.build_dir.as_deref(),
        std::env::var_os("RUVYXA_BUILD_CACHE_DIR"),
    )
}

pub(crate) fn resolve_build_cache_dir(
    root: &Path,
    configured: Option<&str>,
    environment: Option<OsString>,
) -> PathBuf {
    let selected = environment
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            configured
                .filter(|value| !value.trim().is_empty())
                .map(PathBuf::from)
        });

    match selected {
        Some(path) if path.is_absolute() => path,
        Some(path) => root.join(path),
        None => root.join(".ruvyxa").join("cache").join("bundler"),
    }
}

pub(crate) fn diagnostic_stream(value: &str) -> String {
    if value.trim().is_empty() {
        "(empty)".to_string()
    } else {
        value.to_string()
    }
}
