//! TypeScript build-plugin bridge.
//!
//! Ruvyxa's build plugins are written in TypeScript and run in a long-lived
//! JavaScript worker process; the bundler calls them through the synchronous
//! [`BuildHooks`](ruvyxa_bundler::hooks::BuildHooks) trait. This module is the
//! adapter between those two worlds: it owns the worker's lifetime, frames
//! newline-delimited JSON over its stdio, and turns a worker fault into a build
//! error rather than a hang.
//!
//! One worker is shared by every route in a build session, so plugin startup
//! cost is paid once instead of once per route.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command as ProcessCommand, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ruvyxa_dev_server::{JavaScriptRuntime, find_runtime_script};

use crate::BuildPluginConfig;

#[derive(Clone)]
pub(crate) struct TypeScriptPluginBridge {
    pub(crate) project_root: PathBuf,
    pub(crate) workers: Arc<Vec<Mutex<TypeScriptPluginWorker>>>,
    pub(crate) next_worker: Arc<AtomicUsize>,
}

pub(crate) struct TypeScriptPluginWorker {
    pub(crate) child: Child,
    pub(crate) stdin: ChildStdin,
    pub(crate) stdout: BufReader<ChildStdout>,
}

/// Owns the single persistent plugin registry used by one production build.
///
/// Lifecycle and bundler hooks intentionally share this host so config
/// compilation, plugin registration, and process startup happen only once.
pub(crate) struct TypeScriptPluginBuildSession {
    pub(crate) bridge: Option<TypeScriptPluginBridge>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginRuntimeOutput {
    pub(crate) ok: bool,
    pub(crate) result: Option<serde_json::Value>,
    pub(crate) code: Option<String>,
    pub(crate) message: Option<String>,
    pub(crate) stack: Option<String>,
}

impl ruvyxa_bundler::hooks::BuildHooks for TypeScriptPluginBridge {
    fn host_name(&self) -> &str {
        "ruvyxa-typescript-plugin-host"
    }

    fn resolve_id(
        &self,
        specifier: &str,
        importer: Option<&Path>,
        ctx: &ruvyxa_bundler::hooks::BuildHookContext,
    ) -> ruvyxa_bundler::Result<Option<PathBuf>> {
        let payload = serde_json::json!({
            "id": specifier,
            "importer": importer.map(|path| path.display().to_string()),
            "environment": plugin_environment(ctx.target)
        });
        let Some(value) = self.call_runner("build.resolve", payload)? else {
            return Ok(None);
        };
        let Some(path) = value.as_str() else {
            return Ok(None);
        };

        let resolved = PathBuf::from(path);
        let resolved = if resolved.is_absolute() {
            resolved
        } else {
            self.project_root.join(resolved)
        };

        Ok(Some(ruvyxa_diagnostics::normalized_canonical_path(
            &resolved,
        )))
    }

    fn load(
        &self,
        id: &Path,
        ctx: &ruvyxa_bundler::hooks::BuildHookContext,
    ) -> ruvyxa_bundler::Result<Option<ruvyxa_bundler::hooks::TransformOutput>> {
        let payload = serde_json::json!({
            "id": id.display().to_string(),
            "environment": plugin_environment(ctx.target)
        });
        let Some(value) = self.call_runner("build.load", payload)? else {
            return Ok(None);
        };
        let Some(code) = value.get("code").and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        Ok(Some(ruvyxa_bundler::hooks::TransformOutput {
            code: code.to_string(),
            map: value
                .get("map")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
        }))
    }

    fn transform(
        &self,
        code: &str,
        id: &Path,
        ctx: &ruvyxa_bundler::hooks::BuildHookContext,
    ) -> ruvyxa_bundler::Result<Option<ruvyxa_bundler::hooks::TransformOutput>> {
        let payload = serde_json::json!({
            "code": code,
            "id": id.display().to_string(),
            "environment": plugin_environment(ctx.target)
        });
        let Some(value) = self.call_runner("build.transform", payload)? else {
            return Ok(None);
        };
        let Some(code) = value.get("code").and_then(|value| value.as_str()) else {
            return Ok(None);
        };

        let map = value
            .get("map")
            .and_then(|value| value.as_str())
            .map(str::to_string);

        Ok(Some(ruvyxa_bundler::hooks::TransformOutput {
            code: code.to_string(),
            map,
        }))
    }
}

impl TypeScriptPluginBridge {
    pub(crate) fn call_worker(
        &self,
        payload: &serde_json::Value,
    ) -> ruvyxa_bundler::Result<PluginRuntimeOutput> {
        let worker_index = self.next_worker.fetch_add(1, Ordering::Relaxed) % self.workers.len();
        let mut worker = self.workers[worker_index].lock().map_err(|_| {
            ruvyxa_bundler::BundleError::Compiler(
                "TypeScript plugin worker lock was poisoned".into(),
            )
        })?;
        worker.call(payload)
    }

    pub(crate) fn call_runner(
        &self,
        hook: &str,
        mut payload: serde_json::Value,
    ) -> ruvyxa_bundler::Result<Option<serde_json::Value>> {
        payload["hook"] = serde_json::Value::String(hook.to_string());
        let result = self.call_worker(&payload)?;

        if result.ok {
            return Ok(result.result);
        }

        Err(ruvyxa_bundler::BundleError::Compiler(format!(
            "{} {}",
            result.code.unwrap_or_else(|| "RUV1700".to_string()),
            result
                .message
                .or(result.stack)
                .unwrap_or_else(|| "TypeScript plugin hook failed".to_string())
        )))
    }
}

impl TypeScriptPluginBuildSession {
    pub(crate) fn new(
        root: &Path,
        plugins: &[BuildPluginConfig],
        runtime: JavaScriptRuntime,
    ) -> anyhow::Result<Self> {
        if plugins.is_empty() {
            return Ok(Self { bridge: None });
        }

        let runner = find_runtime_script(root, "plugin-runtime.mjs")
            .ok_or_else(|| anyhow::anyhow!("RUV1701 TypeScript plugin runtime not found"))?;
        let project_root = ruvyxa_diagnostics::normalized_canonical_path(root);
        let worker =
            TypeScriptPluginWorker::spawn(&runner, &project_root, runtime).map_err(|error| {
                anyhow::anyhow!("failed to start TypeScript plugin runtime: {error}")
            })?;
        Ok(Self {
            bridge: Some(TypeScriptPluginBridge {
                project_root,
                workers: Arc::new(vec![Mutex::new(worker)]),
                next_worker: Arc::new(AtomicUsize::new(0)),
            }),
        })
    }

    pub(crate) fn bridge(&self) -> Option<&TypeScriptPluginBridge> {
        self.bridge.as_ref()
    }

    pub(crate) fn run_start(&self, out_dir: &Path) -> anyhow::Result<()> {
        self.call_lifecycle(
            "build.start",
            serde_json::json!({ "outDir": out_dir }),
            "build-start",
        )
    }

    pub(crate) fn run_complete(
        &self,
        out_dir: &Path,
        manifest: &serde_json::Value,
    ) -> anyhow::Result<()> {
        self.call_lifecycle(
            "build.complete",
            serde_json::json!({
                "outDir": out_dir,
                "manifest": manifest,
            }),
            "build-complete",
        )
    }

    pub(crate) fn call_lifecycle(
        &self,
        hook: &str,
        mut payload: serde_json::Value,
        label: &str,
    ) -> anyhow::Result<()> {
        let Some(bridge) = &self.bridge else {
            return Ok(());
        };
        payload["hook"] = serde_json::Value::String(hook.to_string());
        let result = bridge
            .call_worker(&payload)
            .map_err(|error| anyhow::anyhow!("TypeScript plugin {label} hook failed: {error}"))?;
        if !result.ok {
            anyhow::bail!(
                "{} {}",
                result.code.unwrap_or_else(|| "RUV1700".to_string()),
                result
                    .message
                    .or(result.stack)
                    .unwrap_or_else(|| format!("TypeScript plugin {label} hook failed"))
            );
        }
        Ok(())
    }
}

impl TypeScriptPluginWorker {
    pub(crate) fn spawn(
        runner: &Path,
        project_root: &Path,
        runtime: JavaScriptRuntime,
    ) -> ruvyxa_bundler::Result<Self> {
        let mut child = ProcessCommand::new(runtime.executable())
            .arg(runner)
            .arg(project_root)
            .arg("--persistent")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // Stdout is reserved for the NDJSON protocol. The runtime routes
            // plugin console output to stderr, so inherit it instead of
            // silently discarding diagnostics during production builds.
            .stderr(Stdio::inherit())
            .env("RUVYXA_RUNTIME", runtime.command())
            .spawn()
            .map_err(|err| {
                ruvyxa_bundler::BundleError::Compiler(format!(
                    "failed to start persistent TypeScript plugin worker: {err}"
                ))
            })?;
        let stdin = child.stdin.take().ok_or_else(|| {
            ruvyxa_bundler::BundleError::Compiler(
                "failed to open TypeScript plugin worker stdin".into(),
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ruvyxa_bundler::BundleError::Compiler(
                "failed to open TypeScript plugin worker stdout".into(),
            )
        })?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    pub(crate) fn call(
        &mut self,
        payload: &serde_json::Value,
    ) -> ruvyxa_bundler::Result<PluginRuntimeOutput> {
        writeln!(self.stdin, "{payload}").map_err(|err| {
            ruvyxa_bundler::BundleError::Compiler(format!(
                "failed to send TypeScript plugin worker payload: {err}"
            ))
        })?;
        self.stdin.flush().map_err(|err| {
            ruvyxa_bundler::BundleError::Compiler(format!(
                "failed to flush TypeScript plugin worker payload: {err}"
            ))
        })?;

        let mut stdout = String::new();
        let bytes_read = self.stdout.read_line(&mut stdout).map_err(|err| {
            ruvyxa_bundler::BundleError::Compiler(format!(
                "failed to read TypeScript plugin worker response: {err}"
            ))
        })?;
        if bytes_read == 0 {
            let status = self
                .child
                .try_wait()
                .ok()
                .flatten()
                .map(|status| status.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            return Err(ruvyxa_bundler::BundleError::Compiler(format!(
                "TypeScript plugin worker exited before responding (status: {status})"
            )));
        }

        serde_json::from_str(stdout.trim()).map_err(|err| {
            ruvyxa_bundler::BundleError::Compiler(format!(
                "TypeScript plugin worker returned invalid output: {err}; stdout: {}",
                stdout.trim()
            ))
        })
    }
}

impl Drop for TypeScriptPluginWorker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub(crate) fn plugin_environment(target: ruvyxa_bundler::BundleTarget) -> &'static str {
    match target {
        ruvyxa_bundler::BundleTarget::Client => "client",
        ruvyxa_bundler::BundleTarget::Ssr => "server",
        ruvyxa_bundler::BundleTarget::Edge => "edge",
    }
}

pub(crate) fn bundle_context_for_build(
    config_dependency_hash: &str,
    cache_dir: &Path,
    plugin_session: &TypeScriptPluginBuildSession,
) -> anyhow::Result<ruvyxa_bundler::BundleContext> {
    let compile_cache = ruvyxa_bundler::cache::CompileCache::at_dir_with_namespace(
        cache_dir,
        true,
        config_dependency_hash,
    );
    let Some(bridge) = plugin_session.bridge() else {
        return Ok(ruvyxa_bundler::BundleContext::for_build(
            compile_cache,
            ruvyxa_bundler::resolver::ResolveGraphCache::for_build(),
            cache_dir,
            config_dependency_hash,
        ));
    };

    Ok(ruvyxa_bundler::BundleContext::with_build_hooks(
        compile_cache,
        ruvyxa_bundler::resolver::ResolveGraphCache::for_build(),
        ruvyxa_bundler::incremental::IncrementalGraphCache::disabled(),
        ruvyxa_bundler::hooks::BuildHookPipeline::new(vec![Arc::new(bridge.clone())]),
    ))
}
