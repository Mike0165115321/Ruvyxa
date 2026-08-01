//! `ruvyxa.config.*` loading, validation, and the JavaScript config renderer.
//!
//! A Ruvyxa config may be TypeScript, so loading it means running a JavaScript
//! renderer and reading the result back as JSON. Every field is then validated
//! *here*, before anything downstream sees it: a limit that is merely wrong — a
//! body cap of zero, a rate-limit window of a billion seconds, a project path
//! that escapes the root — should fail as a config error the user can fix, not
//! as surprising behavior at request time.
//!
//! `deny_unknown_fields` is deliberate. A misspelled key is reported rather
//! than ignored, which is the difference between a security setting that is off
//! and one the user believes is on.

use std::path::{Path, PathBuf};

use ruvyxa_dev_server::{
    JavaScriptRuntime, MAX_ACTION_BODY_LIMIT_BYTES, MAX_ACTION_RATE_LIMIT_REQUESTS,
    MAX_ACTION_RATE_LIMIT_WINDOW_SECS, MAX_API_BODY_LIMIT_BYTES,
    MAX_PLUGIN_RESPONSE_BODY_LIMIT_BYTES, TrustedProxies,
};
use ruvyxa_graph::{DiscoverOptions, RenderStrategy, RouteManifest, discover_routes};

use crate::*;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectConfig {
    pub(crate) app_dir: Option<String>,
    pub(crate) out_dir: Option<String>,
    pub(crate) runtime: Option<BuildTarget>,
    #[serde(rename = "react")]
    pub(crate) _react: Option<serde_json::Value>,
    #[serde(rename = "typescript")]
    pub(crate) _typescript: Option<serde_json::Value>,
    #[serde(default, rename = "render")]
    pub(crate) rendering: RenderingConfigOptions,
    #[serde(default)]
    pub(crate) server: ServerConfigOptions,
    #[serde(default)]
    pub(crate) css: CssConfigOptions,
    #[serde(default)]
    pub(crate) build: BuildConfigOptions,
    #[serde(default)]
    pub(crate) debug: DebugConfigOptions,
    #[serde(default, rename = "image")]
    pub(crate) images: ImageOptimizationOptions,
    #[serde(default)]
    pub(crate) security: SecurityConfigOptions,
    #[serde(default)]
    pub(crate) cache: CacheConfigOptions,
    #[serde(default)]
    pub(crate) site: SiteConfigOptions,
    #[serde(default)]
    pub(crate) middleware: ruvyxa_middleware::MiddlewareConfig,
    #[serde(default)]
    pub(crate) plugins: Vec<BuildPluginConfig>,
    #[serde(rename = "adapter")]
    pub(crate) adapter: Option<serde_json::Value>,
    #[serde(rename = "adapterOptions")]
    pub(crate) adapter_options: Option<serde_json::Value>,
    #[serde(skip)]
    pub(crate) config_dependency_hash: String,
    #[serde(skip)]
    pub(crate) javascript_runtime_override: Option<JavaScriptRuntime>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ServerConfigOptions {
    pub(crate) host: Option<String>,
    pub(crate) port: Option<u16>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CssConfigOptions {
    #[serde(default)]
    pub(crate) entries: Vec<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BuildConfigOptions {
    pub(crate) minify: Option<bool>,
    #[serde(rename = "map")]
    pub(crate) sourcemap: Option<bool>,
    #[serde(rename = "treeShake")]
    pub(crate) tree_shaking: Option<bool>,
    #[serde(rename = "split")]
    pub(crate) split_strategy: Option<String>,
    #[serde(rename = "workers")]
    pub(crate) parallelism: Option<usize>,
    #[serde(rename = "jsx")]
    pub(crate) jsx_runtime: Option<String>,
    #[serde(rename = "target")]
    pub(crate) es_target: Option<String>,
    #[serde(rename = "manifest")]
    pub(crate) emit_chunk_manifest: Option<bool>,
    #[serde(rename = "warm")]
    pub(crate) prebundle_dependencies: Option<bool>,
    #[serde(rename = "prerenderCache")]
    pub(crate) prerender_cache: Option<bool>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RenderingConfigOptions {
    #[serde(rename = "strategy")]
    pub(crate) default_strategy: Option<RenderStrategy>,
    #[serde(rename = "revalidate")]
    pub(crate) default_revalidate: Option<u64>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DebugConfigOptions {
    pub(crate) overlay: Option<bool>,
    pub(crate) traces: Option<bool>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SecurityConfigOptions {
    #[serde(rename = "actionLimit")]
    pub(crate) action_body_limit_bytes: Option<usize>,
    #[serde(rename = "apiLimit")]
    pub(crate) api_body_limit_bytes: Option<usize>,
    #[serde(rename = "pluginLimit")]
    pub(crate) plugin_response_body_limit_bytes: Option<usize>,
    #[serde(rename = "actionRateLimit")]
    pub(crate) action_rate_limit: Option<ActionRateLimitOptions>,
    #[serde(rename = "sameOrigin")]
    pub(crate) same_origin_actions: Option<bool>,
    #[serde(rename = "fetchMeta")]
    pub(crate) fetch_metadata_actions: Option<bool>,
    #[serde(default, rename = "trustedProxyIps")]
    pub(crate) trusted_proxy_ips: Vec<String>,
    #[serde(rename = "headers")]
    pub(crate) security_headers: Option<bool>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActionRateLimitOptions {
    pub(crate) max: Option<usize>,
    pub(crate) window: Option<u64>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CacheConfigOptions {
    #[serde(rename = "routes")]
    pub(crate) route_manifest: Option<bool>,
    pub(crate) css: Option<bool>,
    #[serde(rename = "dir")]
    pub(crate) build_dir: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildPluginConfig {
    pub(crate) name: String,
    /// Elements this plugin contributes to every rendered document's `<head>`.
    #[serde(default)]
    pub(crate) head: Vec<ruvyxa_dev_server::PluginHeadEntry>,
}

pub(crate) struct RuvyxaBuildCache<'a> {
    pub(crate) dependency_hash: &'a str,
    pub(crate) directory: &'a Path,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigRendererOutput {
    pub(crate) ok: bool,
    pub(crate) config: Option<ProjectConfig>,
    pub(crate) code: Option<String>,
    pub(crate) message: Option<String>,
    pub(crate) stack: Option<String>,
    pub(crate) dependency_hash: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdapterRunnerOutput {
    pub(crate) ok: bool,
    pub(crate) result: Option<serde_json::Value>,
    pub(crate) code: Option<String>,
    pub(crate) message: Option<String>,
    pub(crate) stack: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdapterInspection {
    pub(crate) name: String,
    pub(crate) target: String,
    pub(crate) runtime: String,
    pub(crate) platform: Option<String>,
    pub(crate) supports: Vec<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdapterArtifactReport {
    pub(crate) kind: String,
    pub(crate) path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) skipped: Option<bool>,
}

impl ProjectConfig {
    pub(crate) fn build_target(&self, cli_target: Option<BuildTarget>) -> BuildTarget {
        cli_target.or(self.runtime).unwrap_or(BuildTarget::Node)
    }

    pub(crate) fn javascript_runtime(&self) -> JavaScriptRuntime {
        self.javascript_runtime_override
            .unwrap_or_else(|| match self.runtime {
                Some(BuildTarget::Bun) => JavaScriptRuntime::Bun,
                Some(BuildTarget::Node | BuildTarget::Edge | BuildTarget::Static) => {
                    JavaScriptRuntime::Node
                }
                None => JavaScriptRuntime::detect(),
            })
    }

    pub(crate) fn app_dir(&self) -> &str {
        self.app_dir.as_deref().unwrap_or("app")
    }

    pub(crate) fn out_dir(&self) -> &str {
        self.out_dir.as_deref().unwrap_or(".ruvyxa")
    }

    pub(crate) fn validate_paths(&self) -> anyhow::Result<()> {
        validate_project_relative_path("appDir", self.app_dir())?;
        validate_project_relative_path("outDir", self.out_dir())?;
        for entry in &self.css.entries {
            validate_project_relative_path("css.entries", entry)?;
        }
        validate_bounded_limit(
            "security.actionLimit",
            self.security.action_body_limit_bytes,
            MAX_ACTION_BODY_LIMIT_BYTES,
        )?;
        validate_bounded_limit(
            "security.apiLimit",
            self.security.api_body_limit_bytes,
            MAX_API_BODY_LIMIT_BYTES,
        )?;
        validate_plugin_response_limit(self.security.plugin_response_body_limit_bytes)?;
        if let Some(rate_limit) = &self.security.action_rate_limit {
            validate_bounded_limit(
                "security.actionRateLimit.max",
                rate_limit.max,
                MAX_ACTION_RATE_LIMIT_REQUESTS,
            )?;
            validate_bounded_limit(
                "security.actionRateLimit.window",
                rate_limit.window,
                MAX_ACTION_RATE_LIMIT_WINDOW_SECS,
            )?;
        }
        validate_trusted_proxy_ips(&self.security.trusted_proxy_ips)?;
        parse_jsx_runtime(self.build.jsx_runtime.as_deref())?;
        Ok(())
    }

    pub(crate) fn style_entries(&self, root: &Path) -> Vec<PathBuf> {
        let root = ruvyxa_diagnostics::normalized_canonical_path(root);
        self.css
            .entries
            .iter()
            .map(|entry| root.join(entry))
            .collect()
    }

    pub(crate) fn discover_options(&self, root: &Path) -> DiscoverOptions {
        DiscoverOptions::new(root.join(self.app_dir())).with_rendering_defaults(
            self.rendering.default_strategy,
            self.rendering.default_revalidate,
        )
    }
}

pub(crate) fn validate_positive_limit<T>(field: &str, value: Option<T>) -> anyhow::Result<()>
where
    T: PartialEq + From<u8>,
{
    if value.is_some_and(|value| value == T::from(0)) {
        anyhow::bail!("RUV1601 config field `{field}` must be greater than zero");
    }
    Ok(())
}

pub(crate) fn validate_bounded_limit<T>(
    field: &str,
    value: Option<T>,
    maximum: T,
) -> anyhow::Result<()>
where
    T: PartialOrd + PartialEq + From<u8> + std::fmt::Display + Copy,
{
    if let Some(value) = value {
        if value == T::from(0) {
            anyhow::bail!("RUV1601 config field `{field}` must be greater than zero");
        }
        if value > maximum {
            anyhow::bail!("RUV1602 config field `{field}` must not exceed {maximum}");
        }
    }
    Ok(())
}

pub(crate) fn validate_plugin_response_limit(value: Option<usize>) -> anyhow::Result<()> {
    validate_positive_limit("security.pluginLimit", value)?;
    if value.is_some_and(|value| value > MAX_PLUGIN_RESPONSE_BODY_LIMIT_BYTES) {
        anyhow::bail!(
            "RUV1602 config field `security.pluginLimit` must not exceed {MAX_PLUGIN_RESPONSE_BODY_LIMIT_BYTES} bytes"
        );
    }
    Ok(())
}

pub(crate) fn validate_trusted_proxy_ips(values: &[String]) -> anyhow::Result<()> {
    parse_trusted_proxies(values).map(|_| ())
}

/// Parse `security.trustedProxyIps` into matchable prefixes.
///
/// Accepts a CIDR range or a bare address, which is what the field has always
/// been documented to take. Parsing only exact `IpAddr` values rejected every
/// documented example (`10.0.0.0/8`) at startup with `RUV1602`, and left users
/// on container networks and managed platform edges — where the proxy address
/// is not stable enough to enumerate — with no way to declare their proxy at
/// all. Both server builders share this function so validation and the value
/// the server actually uses can never disagree.
pub(crate) fn parse_trusted_proxies(values: &[String]) -> anyhow::Result<TrustedProxies> {
    TrustedProxies::parse_all(values.iter().map(String::as_str)).map_err(|error| {
        anyhow::anyhow!("RUV1602 config field `security.trustedProxyIps` contains {error}")
    })
}

pub(crate) fn discover_project_routes(
    root: &Path,
    config: &ProjectConfig,
) -> anyhow::Result<RouteManifest> {
    discover_routes(config.discover_options(root)).map_err(Into::into)
}

pub(crate) fn validate_project_relative_path(field: &str, value: &str) -> anyhow::Result<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("RUV1601 config field `{field}` must not be empty");
    }

    let path = Path::new(trimmed);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::Prefix(_)
                    | std::path::Component::RootDir
                    | std::path::Component::ParentDir
            )
        })
    {
        anyhow::bail!(
            "RUV1601 config field `{field}` must be a project-relative path inside the project root"
        );
    }

    Ok(())
}
