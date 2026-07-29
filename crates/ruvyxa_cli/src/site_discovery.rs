//! Production `robots.txt` and `sitemap.xml` generation.
//!
//! The build emits conservative crawler-discovery files from the route
//! manifest. Project-owned files copied from `public/` always win. The
//! generator additionally enforces the protocol constraints that are easy to
//! miss in hand-written output: absolute URLs, UTF-8 XML escaping, deterministic
//! ordering, and sitemap sharding at the protocol limits.

use std::collections::BTreeSet;
use std::fs;
use std::io::{Error, ErrorKind};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::Path;

use ruvyxa_graph::{RouteKind, RouteManifest};

const SITE_URL_ENV_VAR: &str = "RUVYXA_SITE_URL";
const VERCEL_PRODUCTION_URL_ENV_VAR: &str = "VERCEL_PROJECT_PRODUCTION_URL";
const VERCEL_DEPLOYMENT_URL_ENV_VAR: &str = "VERCEL_URL";
const NETLIFY_URL_ENV_VAR: &str = "URL";
const SITEMAP_MAX_URLS: usize = 50_000;
const SITEMAP_MAX_BYTES: usize = 52_428_800;
const SITEMAP_MAX_LOCATION_CHARS: usize = 2_048;
const SITEMAP_HEADER: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n";
const SITEMAP_FOOTER: &str = "</urlset>\n";

/// `site` block of `ruvyxa.config.ts`.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SiteConfigOptions {
    /// Absolute origin of the deployed site, e.g. `https://ruvyxa.dev`.
    pub url: Option<String>,
    /// Sitemap generation switch or production options. @default true
    #[serde(default)]
    pub sitemap: SitemapSetting,
    /// Robots generation switch or production options. @default true
    #[serde(default)]
    pub robots: RobotsSetting,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum SitemapSetting {
    Enabled(bool),
    Options(SitemapGenerationOptions),
}

impl Default for SitemapSetting {
    fn default() -> Self {
        Self::Enabled(true)
    }
}

impl SitemapSetting {
    fn enabled(&self) -> bool {
        !matches!(self, Self::Enabled(false))
    }

    fn options(&self) -> Option<&SitemapGenerationOptions> {
        match self {
            Self::Options(options) => Some(options),
            Self::Enabled(_) => None,
        }
    }
}

/// Additive controls for the route-derived sitemap.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SitemapGenerationOptions {
    /// Exact paths or trailing-`*` path prefixes omitted from the sitemap.
    #[serde(default)]
    exclude: Vec<String>,
    /// Concrete paths that cannot be inferred from the route manifest.
    #[serde(default)]
    additional_paths: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum RobotsSetting {
    Enabled(bool),
    Options(RobotsGenerationOptions),
}

impl Default for RobotsSetting {
    fn default() -> Self {
        Self::Enabled(true)
    }
}

impl RobotsSetting {
    fn enabled(&self) -> bool {
        !matches!(self, Self::Enabled(false))
    }

    fn options(&self) -> Option<&RobotsGenerationOptions> {
        match self {
            Self::Options(options) => Some(options),
            Self::Enabled(_) => None,
        }
    }
}

/// Next-style robots policy serialized into the RFC 9309 text format.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RobotsGenerationOptions {
    rules: Option<OneOrManyRules>,
    sitemap: Option<OneOrManyStrings>,
    host: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
enum OneOrManyRules {
    One(RobotsRuleOptions),
    Many(Vec<RobotsRuleOptions>),
}

impl OneOrManyRules {
    fn values(&self) -> Vec<&RobotsRuleOptions> {
        match self {
            Self::One(value) => vec![value],
            Self::Many(values) => values.iter().collect(),
        }
    }
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RobotsRuleOptions {
    user_agent: Option<OneOrManyStrings>,
    allow: Option<OneOrManyStrings>,
    disallow: Option<OneOrManyStrings>,
    crawl_delay: Option<u64>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
enum OneOrManyStrings {
    One(String),
    Many(Vec<String>),
}

impl OneOrManyStrings {
    fn values(&self) -> Vec<&str> {
        match self {
            Self::One(value) => vec![value],
            Self::Many(values) => values.iter().map(String::as_str).collect(),
        }
    }
}

/// What the build wrote, for the CLI summary and production diagnostics.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DiscoveryReport {
    pub robots_written: bool,
    pub sitemap_written: bool,
    pub sitemap_files_written: usize,
    /// Set when a sitemap was wanted but no site URL could be resolved.
    pub sitemap_needs_site_url: bool,
}

/// Resolve a validated absolute origin from explicit config or production-host
/// environment variables. Preview-specific deployment URLs are never used as
/// canonical sitemap origins.
pub fn resolve_site_url<F>(configured: Option<&str>, env: F) -> Result<Option<String>, String>
where
    F: Fn(&str) -> Option<String>,
{
    if let Some(value) = configured.filter(|value| !value.trim().is_empty()) {
        return normalize_site_origin(value)
            .map(Some)
            .map_err(|message| format!("site.url {message}"));
    }

    let mut candidates = Vec::new();
    if let Some(value) = env(SITE_URL_ENV_VAR).filter(|value| !value.trim().is_empty()) {
        candidates.push((SITE_URL_ENV_VAR, value));
    }
    if let Some(value) = env(VERCEL_PRODUCTION_URL_ENV_VAR).filter(|value| !value.trim().is_empty())
    {
        candidates.push((VERCEL_PRODUCTION_URL_ENV_VAR, value));
    }
    if env("VERCEL_ENV").is_some_and(|value| value.eq_ignore_ascii_case("production"))
        && let Some(value) =
            env(VERCEL_DEPLOYMENT_URL_ENV_VAR).filter(|value| !value.trim().is_empty())
    {
        candidates.push((VERCEL_DEPLOYMENT_URL_ENV_VAR, value));
    }
    if env("NETLIFY").is_some_and(|value| value.eq_ignore_ascii_case("true"))
        && let Some(value) = env(NETLIFY_URL_ENV_VAR).filter(|value| !value.trim().is_empty())
    {
        candidates.push((NETLIFY_URL_ENV_VAR, value));
    }

    let Some((source, value)) = candidates.into_iter().next() else {
        return Ok(None);
    };
    normalize_site_origin(&value)
        .map(Some)
        .map_err(|message| format!("{source} {message}"))
}

/// Write missing crawler-discovery files into the staged public assets.
pub fn write_discovery_files(
    manifest: &RouteManifest,
    prerendered_paths: &[String],
    assets_dir: &Path,
    site_url: Option<&str>,
    options: &SiteConfigOptions,
) -> std::io::Result<DiscoveryReport> {
    let mut report = DiscoveryReport::default();
    let sitemap_path = assets_dir.join("sitemap.xml");
    let sitemap_route_exists = manifest
        .routes
        .iter()
        .any(|route| route.path == "/sitemap.xml");

    if options.sitemap.enabled() && !sitemap_path.exists() && !sitemap_route_exists {
        match site_url {
            Some(url) => {
                let paths =
                    indexable_paths(manifest, prerendered_paths, options.sitemap.options())?;
                let documents = sitemap_documents(&paths, url)?;
                fs::create_dir_all(assets_dir)?;
                if documents.len() == 1 {
                    fs::write(&sitemap_path, &documents[0])?;
                    report.sitemap_files_written = 1;
                } else {
                    for (index, document) in documents.iter().enumerate() {
                        let shard_path = assets_dir.join(format!("sitemap-{index}.xml"));
                        if shard_path.exists() {
                            return Err(invalid_input(format!(
                                "generated sitemap shard would overwrite project file {}",
                                shard_path.display()
                            )));
                        }
                        fs::write(shard_path, document)?;
                    }
                    fs::write(&sitemap_path, sitemap_index_xml(url, documents.len())?)?;
                    report.sitemap_files_written = documents.len() + 1;
                }
                report.sitemap_written = true;
            }
            None => report.sitemap_needs_site_url = true,
        }
    }

    let robots_path = assets_dir.join("robots.txt");
    let robots_route_exists = manifest
        .routes
        .iter()
        .any(|route| route.path == "/robots.txt");
    if options.robots.enabled() && !robots_path.exists() && !robots_route_exists {
        fs::create_dir_all(assets_dir)?;
        let automatic_sitemap = site_url
            .filter(|_| report.sitemap_written || sitemap_path.exists() || sitemap_route_exists);
        fs::write(
            &robots_path,
            robots_txt(automatic_sitemap, options.robots.options())?,
        )?;
        report.robots_written = true;
    }

    Ok(report)
}

fn indexable_paths(
    manifest: &RouteManifest,
    prerendered_paths: &[String],
    options: Option<&SitemapGenerationOptions>,
) -> std::io::Result<Vec<String>> {
    let exclusions = options.map_or(&[][..], |options| options.exclude.as_slice());
    for pattern in exclusions {
        validate_exclusion_pattern(pattern)?;
    }

    let mut paths = manifest
        .routes
        .iter()
        .filter(|route| route.kind == RouteKind::Page && !route.path.contains('['))
        .map(|route| route.path.clone())
        .chain(
            prerendered_paths
                .iter()
                .filter(|path| !path.contains('['))
                .cloned(),
        )
        .collect::<BTreeSet<_>>();

    if let Some(options) = options {
        for path in &options.additional_paths {
            validate_application_path(path, "site.sitemap.additionalPaths")?;
            paths.insert(path.clone());
        }
    }

    paths.retain(|path| {
        !exclusions
            .iter()
            .any(|pattern| exclusion_matches(pattern, path))
    });
    Ok(paths.into_iter().collect())
}

fn sitemap_documents(paths: &[String], site_url: &str) -> std::io::Result<Vec<String>> {
    sitemap_documents_with_limits(paths, site_url, SITEMAP_MAX_URLS, SITEMAP_MAX_BYTES)
}

fn sitemap_documents_with_limits(
    paths: &[String],
    site_url: &str,
    max_urls: usize,
    max_bytes: usize,
) -> std::io::Result<Vec<String>> {
    if max_urls == 0 || max_bytes <= SITEMAP_HEADER.len() + SITEMAP_FOOTER.len() {
        return Err(invalid_input("sitemap limits must allow at least one URL"));
    }

    let mut documents = Vec::new();
    let mut document = String::from(SITEMAP_HEADER);
    let mut url_count = 0usize;

    for path in paths {
        validate_application_path(path, "sitemap route")?;
        let location = format!("{site_url}{}", percent_encode_path(path));
        if location.chars().count() > SITEMAP_MAX_LOCATION_CHARS {
            return Err(invalid_input(format!(
                "sitemap URL exceeds {SITEMAP_MAX_LOCATION_CHARS} characters: {location}"
            )));
        }
        let entry = format!("  <url><loc>{}</loc></url>\n", escape_xml(&location));
        let exceeds_bytes = document.len() + entry.len() + SITEMAP_FOOTER.len() > max_bytes;
        if url_count > 0 && (url_count == max_urls || exceeds_bytes) {
            document.push_str(SITEMAP_FOOTER);
            documents.push(document);
            document = String::from(SITEMAP_HEADER);
            url_count = 0;
        }
        if document.len() + entry.len() + SITEMAP_FOOTER.len() > max_bytes {
            return Err(invalid_input(format!(
                "one sitemap entry exceeds the {max_bytes}-byte document limit"
            )));
        }
        document.push_str(&entry);
        url_count += 1;
    }

    document.push_str(SITEMAP_FOOTER);
    documents.push(document);
    Ok(documents)
}

fn sitemap_index_xml(site_url: &str, shard_count: usize) -> std::io::Result<String> {
    if shard_count > SITEMAP_MAX_URLS {
        return Err(invalid_input(format!(
            "sitemap index exceeds the {SITEMAP_MAX_URLS}-entry protocol limit"
        )));
    }
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for index in 0..shard_count {
        let location = format!("{site_url}/sitemap-{index}.xml");
        if location.chars().count() > SITEMAP_MAX_LOCATION_CHARS {
            return Err(invalid_input("generated sitemap shard URL is too long"));
        }
        xml.push_str("  <sitemap><loc>");
        xml.push_str(&escape_xml(&location));
        xml.push_str("</loc></sitemap>\n");
    }
    xml.push_str("</sitemapindex>\n");
    if xml.len() > SITEMAP_MAX_BYTES {
        return Err(invalid_input(format!(
            "sitemap index exceeds the {SITEMAP_MAX_BYTES}-byte protocol limit"
        )));
    }
    Ok(xml)
}

fn robots_txt(
    automatic_sitemap_origin: Option<&str>,
    options: Option<&RobotsGenerationOptions>,
) -> std::io::Result<String> {
    let default_rule = RobotsRuleOptions {
        user_agent: Some(OneOrManyStrings::One("*".to_string())),
        allow: Some(OneOrManyStrings::One("/".to_string())),
        ..RobotsRuleOptions::default()
    };
    let rules = options
        .and_then(|options| options.rules.as_ref())
        .map(OneOrManyRules::values)
        .filter(|rules| !rules.is_empty())
        .unwrap_or_else(|| vec![&default_rule]);
    let mut blocks = Vec::new();

    for rule in rules {
        let agents = rule
            .user_agent
            .as_ref()
            .map_or_else(|| vec!["*"], OneOrManyStrings::values);
        if agents.is_empty() {
            return Err(invalid_input(
                "site.robots.rules userAgent must not be empty",
            ));
        }
        for agent in agents {
            validate_user_agent(agent)?;
            let mut lines = vec![format!("User-agent: {agent}")];
            append_robots_paths(&mut lines, "Allow", rule.allow.as_ref())?;
            append_robots_paths(&mut lines, "Disallow", rule.disallow.as_ref())?;
            if let Some(delay) = rule.crawl_delay {
                lines.push(format!("Crawl-delay: {delay}"));
            }
            blocks.push(lines.join("\n"));
        }
    }

    let mut text = blocks.join("\n\n");
    text.push('\n');
    let explicit_sitemaps = options.and_then(|options| options.sitemap.as_ref());
    let sitemap_urls = match explicit_sitemaps {
        Some(values) => values
            .values()
            .into_iter()
            .map(|value| normalize_absolute_http_url(value, "site.robots.sitemap"))
            .collect::<std::io::Result<Vec<_>>>()?,
        None => automatic_sitemap_origin
            .map(|origin| vec![format!("{origin}/sitemap.xml")])
            .unwrap_or_default(),
    };
    if !sitemap_urls.is_empty() {
        text.push('\n');
        for sitemap in sitemap_urls {
            text.push_str(&format!("Sitemap: {sitemap}\n"));
        }
    }
    if let Some(host) = options.and_then(|options| options.host.as_deref()) {
        let host = normalize_site_origin(host).map_err(invalid_input)?;
        text.push_str(&format!("Host: {host}\n"));
    }
    Ok(text)
}

fn append_robots_paths(
    lines: &mut Vec<String>,
    directive: &str,
    values: Option<&OneOrManyStrings>,
) -> std::io::Result<()> {
    for value in values.map(OneOrManyStrings::values).unwrap_or_default() {
        if !value.is_empty() && !value.starts_with('/') {
            return Err(invalid_input(format!(
                "site.robots.rules {directive} value must start with '/': {value}"
            )));
        }
        validate_single_line(value, "site.robots.rules path")?;
        lines.push(format!("{directive}: {value}"));
    }
    Ok(())
}

fn validate_user_agent(value: &str) -> std::io::Result<()> {
    if value == "*"
        || (!value.is_empty()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphabetic() || matches!(byte, b'-' | b'_')))
    {
        return Ok(());
    }
    Err(invalid_input(format!(
        "site.robots.rules userAgent is not a valid product token: {value}"
    )))
}

fn normalize_site_origin(value: &str) -> Result<String, String> {
    let value = value.trim();
    validate_url_text(value).map_err(|message| message.to_string())?;
    let normalized = if value.contains("://") {
        value.to_string()
    } else {
        format!("https://{value}")
    };
    let (scheme, remainder) = normalized
        .split_once("://")
        .ok_or_else(|| "must be an absolute http(s) origin".to_string())?;
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") {
        return Err("must use the http or https scheme".to_string());
    }
    let remainder = if let Some(origin) = remainder.strip_suffix('/') {
        if origin.ends_with('/') {
            return Err("must be an origin without a path, query, or fragment".to_string());
        }
        origin
    } else {
        remainder
    };
    if remainder.contains(['/', '?', '#']) {
        return Err("must be an origin without a path, query, or fragment".to_string());
    }
    let authority = normalize_authority(remainder)?;
    Ok(format!("{}://{authority}", scheme.to_ascii_lowercase()))
}

fn normalize_absolute_http_url(value: &str, field: &str) -> std::io::Result<String> {
    let value = value.trim();
    validate_url_text(value).map_err(invalid_input)?;
    let (scheme, remainder) = value
        .split_once("://")
        .ok_or_else(|| invalid_input(format!("{field} must be an absolute http(s) URL")))?;
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") {
        return Err(invalid_input(format!("{field} must use http or https")));
    }
    if remainder.contains('#') {
        return Err(invalid_input(format!(
            "{field} must not contain a fragment"
        )));
    }
    let boundary = remainder.find(['/', '?']).unwrap_or(remainder.len());
    let authority = normalize_authority(&remainder[..boundary]).map_err(invalid_input)?;
    Ok(format!(
        "{}://{authority}{}",
        scheme.to_ascii_lowercase(),
        &remainder[boundary..]
    ))
}

fn normalize_authority(value: &str) -> Result<String, String> {
    if value.is_empty() || value.contains('@') {
        return Err("must contain a host and must not contain credentials".to_string());
    }

    if let Some(address) = value.strip_prefix('[') {
        let closing = address
            .find(']')
            .ok_or_else(|| "contains an invalid IPv6 host".to_string())?;
        let host = &address[..closing];
        let suffix = &address[closing + 1..];
        let parsed = host
            .parse::<Ipv6Addr>()
            .map_err(|_| "contains an invalid IPv6 host".to_string())?;
        let port = normalize_port_suffix(suffix)?;
        return Ok(format!("[{parsed}]{port}"));
    }

    if value.matches(':').count() > 1 {
        return Err("IPv6 hosts must use brackets".to_string());
    }
    let (host, port) = value
        .rsplit_once(':')
        .map_or((value, String::new()), |(host, port)| {
            (host, format!(":{port}"))
        });
    let port = normalize_port_suffix(&port)?;
    if host.is_empty() {
        return Err("must contain a host".to_string());
    }
    if host.parse::<Ipv4Addr>().is_err() {
        validate_dns_host(host)?;
    }
    Ok(format!("{}{port}", host.to_ascii_lowercase()))
}

fn normalize_port_suffix(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Ok(String::new());
    }
    let port = value
        .strip_prefix(':')
        .ok_or_else(|| "contains invalid text after the host".to_string())?;
    let port = port
        .parse::<u16>()
        .map_err(|_| "contains an invalid port".to_string())?;
    if port == 0 {
        return Err("contains an invalid port".to_string());
    }
    Ok(format!(":{port}"))
}

fn validate_dns_host(host: &str) -> Result<(), String> {
    if host.len() > 253
        || host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return Err("contains an invalid DNS host; use an ASCII or punycode hostname".to_string());
    }
    Ok(())
}

fn validate_url_text(value: &str) -> Result<(), &'static str> {
    if value.is_empty() {
        return Err("must not be empty");
    }
    if value
        .chars()
        .any(|character| character.is_whitespace() || character.is_control() || character == '\\')
    {
        return Err("contains whitespace, a control character, or a backslash");
    }
    Ok(())
}

fn validate_application_path(path: &str, field: &str) -> std::io::Result<()> {
    if !path.starts_with('/')
        || path.contains(['\\', '?', '#', '[', ']'])
        || path.chars().any(char::is_control)
        || path.split('/').any(|segment| matches!(segment, "." | ".."))
    {
        return Err(invalid_input(format!(
            "{field} must be a concrete absolute application path: {path}"
        )));
    }
    Ok(())
}

fn validate_exclusion_pattern(pattern: &str) -> std::io::Result<()> {
    let path = pattern.strip_suffix('*').unwrap_or(pattern);
    if path.contains('*') {
        return Err(invalid_input(format!(
            "site.sitemap.exclude only supports a trailing '*': {pattern}"
        )));
    }
    validate_application_path(path, "site.sitemap.exclude")
}

fn exclusion_matches(pattern: &str, path: &str) -> bool {
    pattern
        .strip_suffix('*')
        .map_or(path == pattern, |prefix| path.starts_with(prefix))
}

fn validate_single_line(value: &str, field: &str) -> std::io::Result<()> {
    if value.chars().any(char::is_control) {
        return Err(invalid_input(format!(
            "{field} must not contain control characters"
        )));
    }
    Ok(())
}

fn percent_encode_path(path: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte == b'/' || byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
        {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    encoded
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn invalid_input(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidInput, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruvyxa_graph::{RenderMeta, RouteEntry, RuntimeTarget};
    use std::path::PathBuf;

    fn manifest(paths: &[(&str, RouteKind)]) -> RouteManifest {
        RouteManifest {
            app_dir: PathBuf::from("/project/app"),
            routes: paths
                .iter()
                .map(|(path, kind)| RouteEntry {
                    id: path.to_string(),
                    path: path.to_string(),
                    kind: *kind,
                    file: PathBuf::from("/project/app/page.tsx"),
                    layout_chain: Vec::new(),
                    server_modules: Vec::new(),
                    client_modules: Vec::new(),
                    runtime: RuntimeTarget::Node,
                    render: RenderMeta::default(),
                })
                .collect(),
        }
    }

    fn no_env(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn site_url_prefers_config_and_uses_production_host_fallbacks() {
        assert_eq!(
            resolve_site_url(Some("HTTPS://Ruvyxa.dev/"), |_| Some(
                "https://env".to_string()
            )),
            Ok(Some("https://ruvyxa.dev".to_string()))
        );
        assert_eq!(
            resolve_site_url(Some("Ruvyxa.dev"), no_env),
            Ok(Some("https://ruvyxa.dev".to_string()))
        );
        assert_eq!(
            resolve_site_url(None, |name| (name == "VERCEL_PROJECT_PRODUCTION_URL")
                .then(|| "demo.ruvyxa.dev".to_string())),
            Ok(Some("https://demo.ruvyxa.dev".to_string()))
        );
        assert_eq!(
            resolve_site_url(None, |name| match name {
                "NETLIFY" => Some("true".to_string()),
                "URL" => Some("https://ruvyxa.netlify.app".to_string()),
                _ => None,
            }),
            Ok(Some("https://ruvyxa.netlify.app".to_string()))
        );
        assert_eq!(resolve_site_url(None, no_env), Ok(None));
    }

    #[test]
    fn site_url_rejects_non_origin_and_credential_values() {
        for value in [
            "ftp://ruvyxa.dev",
            "https://ruvyxa.dev/docs",
            "https://user@ruvyxa.dev",
            "https://ruvyxa.dev?preview=1",
            "https://ruvyxa.dev///",
            "https://",
        ] {
            assert!(resolve_site_url(Some(value), no_env).is_err(), "{value}");
        }
        assert_eq!(
            resolve_site_url(Some("http://[::1]:3000"), no_env),
            Ok(Some("http://[::1]:3000".to_string()))
        );
    }

    #[test]
    fn sitemap_lists_prerendered_and_additional_paths_with_encoding_and_exclusions() {
        let manifest = manifest(&[
            ("/", RouteKind::Page),
            ("/about", RouteKind::Page),
            ("/[lang]/docs/[slug]", RouteKind::Page),
            ("/drafts/secret", RouteKind::Page),
        ]);
        let setting = SitemapGenerationOptions {
            exclude: vec!["/drafts/*".to_string()],
            additional_paths: vec!["/products/ชาไทย".to_string()],
        };
        let paths = indexable_paths(
            &manifest,
            &["/en/docs/routing & metadata".to_string(), "/".to_string()],
            Some(&setting),
        )
        .unwrap();
        let xml = &sitemap_documents(&paths, "https://ruvyxa.dev").unwrap()[0];

        assert_eq!(xml.matches("<loc>").count(), 4, "{xml}");
        assert!(
            xml.contains("https://ruvyxa.dev/en/docs/routing%20%26%20metadata"),
            "{xml}"
        );
        assert!(xml.contains("/products/%E0%B8%8A%E0%B8%B2%E0%B9%84%E0%B8%97%E0%B8%A2"));
        assert!(!xml.contains("drafts"), "{xml}");
        assert!(!xml.contains('['), "{xml}");
    }

    #[test]
    fn sitemap_lists_static_pages_only() {
        let manifest = manifest(&[
            ("/", RouteKind::Page),
            ("/blog", RouteKind::Page),
            ("/blog/[slug]", RouteKind::Page),
            ("/api/health", RouteKind::Api),
        ]);
        let paths = indexable_paths(&manifest, &[], None).unwrap();
        let xml = &sitemap_documents(&paths, "https://ruvyxa.dev").unwrap()[0];
        assert!(xml.contains("<loc>https://ruvyxa.dev/</loc>"), "{xml}");
        assert!(xml.contains("<loc>https://ruvyxa.dev/blog</loc>"), "{xml}");
        assert!(!xml.contains("[slug]"), "{xml}");
        assert!(!xml.contains("/api/health"), "{xml}");
    }

    #[test]
    fn sitemap_shards_by_protocol_limits_and_writes_an_index() {
        let paths = vec!["/a".to_string(), "/b".to_string(), "/c".to_string()];
        let documents =
            sitemap_documents_with_limits(&paths, "https://ruvyxa.dev", 2, 10_000).unwrap();
        assert_eq!(documents.len(), 2);
        assert_eq!(documents[0].matches("<url>").count(), 2);
        assert_eq!(documents[1].matches("<url>").count(), 1);
        let index = sitemap_index_xml("https://ruvyxa.dev", documents.len()).unwrap();
        assert!(index.contains("https://ruvyxa.dev/sitemap-0.xml"));
        assert!(index.contains("https://ruvyxa.dev/sitemap-1.xml"));
    }

    #[test]
    fn generated_files_never_overwrite_project_files() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        fs::create_dir_all(&assets).unwrap();
        fs::write(assets.join("robots.txt"), "User-agent: *\nDisallow: /\n").unwrap();
        fs::write(assets.join("sitemap.xml"), "<urlset/>").unwrap();

        let report = write_discovery_files(
            &manifest(&[("/", RouteKind::Page)]),
            &[],
            &assets,
            Some("https://ruvyxa.dev"),
            &SiteConfigOptions::default(),
        )
        .unwrap();
        assert_eq!(report, DiscoveryReport::default());
        assert_eq!(
            fs::read_to_string(assets.join("robots.txt")).unwrap(),
            "User-agent: *\nDisallow: /\n"
        );
        assert_eq!(
            fs::read_to_string(assets.join("sitemap.xml")).unwrap(),
            "<urlset/>"
        );
    }

    #[test]
    fn explicit_metadata_routes_prevent_static_generation() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        let report = write_discovery_files(
            &manifest(&[
                ("/", RouteKind::Page),
                ("/sitemap.xml", RouteKind::Api),
                ("/robots.txt", RouteKind::Api),
            ]),
            &[],
            &assets,
            Some("https://ruvyxa.dev"),
            &SiteConfigOptions::default(),
        )
        .unwrap();

        assert_eq!(report, DiscoveryReport::default());
        assert!(!assets.join("robots.txt").exists());
        assert!(!assets.join("sitemap.xml").exists());
    }

    #[test]
    fn robots_supports_next_style_rules_multiple_sitemaps_and_host() {
        let options = RobotsGenerationOptions {
            rules: Some(OneOrManyRules::One(RobotsRuleOptions {
                user_agent: Some(OneOrManyStrings::Many(vec![
                    "Googlebot".to_string(),
                    "Bingbot".to_string(),
                ])),
                allow: Some(OneOrManyStrings::One("/".to_string())),
                disallow: Some(OneOrManyStrings::Many(vec![
                    "/private/".to_string(),
                    "/drafts/*".to_string(),
                ])),
                crawl_delay: Some(5),
            })),
            sitemap: Some(OneOrManyStrings::Many(vec![
                "https://ruvyxa.dev/sitemap.xml".to_string(),
                "https://ruvyxa.dev/news-sitemap.xml".to_string(),
            ])),
            host: Some("ruvyxa.dev".to_string()),
        };
        let text = robots_txt(None, Some(&options)).unwrap();
        assert!(text.contains("User-agent: Googlebot\nAllow: /\nDisallow: /private/"));
        assert!(text.contains("User-agent: Bingbot"));
        assert_eq!(text.matches("Sitemap:").count(), 2);
        assert!(text.contains("Host: https://ruvyxa.dev"));
    }

    #[test]
    fn robots_rejects_line_injection_and_invalid_paths() {
        let invalid_agent = RobotsGenerationOptions {
            rules: Some(OneOrManyRules::One(RobotsRuleOptions {
                user_agent: Some(OneOrManyStrings::One("Bot\nDisallow: /".to_string())),
                ..RobotsRuleOptions::default()
            })),
            ..RobotsGenerationOptions::default()
        };
        assert!(robots_txt(None, Some(&invalid_agent)).is_err());

        let invalid_path = RobotsGenerationOptions {
            rules: Some(OneOrManyRules::One(RobotsRuleOptions {
                disallow: Some(OneOrManyStrings::One("private".to_string())),
                ..RobotsRuleOptions::default()
            })),
            ..RobotsGenerationOptions::default()
        };
        assert!(robots_txt(None, Some(&invalid_path)).is_err());
    }

    #[test]
    fn robots_is_written_without_a_site_url_and_omits_the_sitemap_line() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        let report = write_discovery_files(
            &manifest(&[("/", RouteKind::Page)]),
            &[],
            &assets,
            None,
            &SiteConfigOptions::default(),
        )
        .unwrap();
        assert!(report.robots_written);
        assert!(!report.sitemap_written);
        assert!(report.sitemap_needs_site_url);
        assert_eq!(
            fs::read_to_string(assets.join("robots.txt")).unwrap(),
            "User-agent: *\nAllow: /\n"
        );
        assert!(!assets.join("sitemap.xml").exists());
    }

    #[test]
    fn a_full_run_links_robots_to_the_generated_sitemap() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        let report = write_discovery_files(
            &manifest(&[("/", RouteKind::Page), ("/about", RouteKind::Page)]),
            &[],
            &assets,
            Some("https://ruvyxa.dev"),
            &SiteConfigOptions::default(),
        )
        .unwrap();
        assert!(report.robots_written && report.sitemap_written);
        assert_eq!(report.sitemap_files_written, 1);
        assert!(
            fs::read_to_string(assets.join("robots.txt"))
                .unwrap()
                .contains("Sitemap: https://ruvyxa.dev/sitemap.xml")
        );
    }

    #[test]
    fn each_file_can_be_disabled_independently() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        let report = write_discovery_files(
            &manifest(&[("/", RouteKind::Page)]),
            &[],
            &assets,
            Some("https://ruvyxa.dev"),
            &SiteConfigOptions {
                url: None,
                sitemap: SitemapSetting::Enabled(false),
                robots: RobotsSetting::Enabled(false),
            },
        )
        .unwrap();
        assert_eq!(report, DiscoveryReport::default());
        assert!(!assets.join("robots.txt").exists());
        assert!(!assets.join("sitemap.xml").exists());
    }
}
