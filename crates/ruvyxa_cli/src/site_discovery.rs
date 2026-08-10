//! Production `robots.txt` and `sitemap.xml` generation.
//!
//! The build emits conservative crawler-discovery files from the route
//! manifest. Project-owned files copied from `public/` always win. The
//! generator additionally enforces the protocol constraints that are easy to
//! miss in hand-written output: absolute URLs, UTF-8 XML escaping, deterministic
//! ordering, and sitemap sharding at the protocol limits.

use std::collections::BTreeMap;
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
const SITEMAP_XML_DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
const SITEMAP_FOOTER: &str = "</urlset>\n";

/// `site` block of `ruvyxa.config.ts`.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SiteConfigOptions {
    /// Absolute origin of the deployed site, e.g. `https://ruvyxa.dev`.
    pub url: Option<String>,
    /// Shared title consumed by content-derived artifacts.
    #[serde(rename = "title")]
    pub _title: Option<String>,
    /// Shared description consumed by content-derived artifacts.
    #[serde(rename = "description")]
    pub _description: Option<String>,
    /// BCP 47 language consumed by feeds and content tokenization.
    #[serde(rename = "language")]
    pub _language: Option<String>,
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
    /// Metadata applied to every automatically discovered or explicit entry.
    #[serde(default)]
    defaults: SitemapEntryMetadata,
    /// Next-style entries that enrich discovered URLs or add new URLs.
    #[serde(default)]
    entries: Vec<SitemapEntryOptions>,
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SitemapEntryMetadata {
    last_modified: Option<String>,
    change_frequency: Option<SitemapChangeFrequency>,
    priority: Option<f64>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SitemapEntryOptions {
    url: String,
    last_modified: Option<String>,
    change_frequency: Option<SitemapChangeFrequency>,
    priority: Option<f64>,
    #[serde(default)]
    alternates: SitemapAlternates,
    #[serde(default)]
    images: Vec<String>,
    #[serde(default)]
    videos: Vec<SitemapVideo>,
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SitemapAlternates {
    #[serde(default)]
    languages: BTreeMap<String, String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct SitemapVideo {
    title: String,
    thumbnail_loc: String,
    description: String,
    content_loc: Option<String>,
    player_loc: Option<String>,
    duration: Option<u32>,
    view_count: Option<u64>,
    rating: Option<f64>,
    expiration_date: Option<String>,
    publication_date: Option<String>,
    family_friendly: Option<YesNo>,
    requires_subscription: Option<YesNo>,
    live: Option<YesNo>,
    restriction: Option<SitemapVideoRelationship>,
    platform: Option<SitemapVideoRelationship>,
    uploader: Option<SitemapVideoUploader>,
    tag: Option<OneOrManyStrings>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum YesNo {
    Yes,
    No,
}

impl YesNo {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Yes => "yes",
            Self::No => "no",
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SitemapVideoRelationship {
    relationship: SitemapRelationship,
    content: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum SitemapRelationship {
    Allow,
    Deny,
}

impl SitemapRelationship {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SitemapVideoUploader {
    content: String,
    info: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum SitemapChangeFrequency {
    Always,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Never,
}

impl SitemapChangeFrequency {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Yearly => "yearly",
            Self::Never => "never",
        }
    }
}

#[derive(Debug, Default, Clone)]
struct SitemapEntry {
    location: String,
    last_modified: Option<String>,
    change_frequency: Option<SitemapChangeFrequency>,
    priority: Option<f64>,
    alternates: BTreeMap<String, String>,
    images: Vec<String>,
    videos: Vec<SitemapVideo>,
}

#[derive(Debug, Default, Clone, Copy)]
struct SitemapFeatures {
    alternates: bool,
    images: bool,
    videos: bool,
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
                let entries =
                    sitemap_entries(manifest, prerendered_paths, url, options.sitemap.options())?;
                let documents = sitemap_documents(&entries)?;
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

fn sitemap_entries(
    manifest: &RouteManifest,
    prerendered_paths: &[String],
    site_url: &str,
    options: Option<&SitemapGenerationOptions>,
) -> std::io::Result<Vec<SitemapEntry>> {
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
        .collect::<std::collections::BTreeSet<_>>();

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
    let defaults = options.map_or_else(SitemapEntryMetadata::default, |options| {
        options.defaults.clone()
    });
    validate_entry_metadata(&defaults, "site.sitemap.defaults")?;
    let mut entries = BTreeMap::new();
    for path in paths {
        let location = format!("{site_url}{}", percent_encode_path(&path));
        validate_sitemap_location(&location, "sitemap route")?;
        entries.insert(
            location.clone(),
            SitemapEntry {
                location,
                last_modified: defaults.last_modified.clone(),
                change_frequency: defaults.change_frequency.clone(),
                priority: defaults.priority,
                ..SitemapEntry::default()
            },
        );
    }

    if let Some(options) = options {
        for (index, configured) in options.entries.iter().enumerate() {
            let field = format!("site.sitemap.entries[{index}]");
            let location = normalize_sitemap_entry_url(&configured.url, site_url, &field)?;
            let mut entry = entries.remove(&location).unwrap_or_else(|| SitemapEntry {
                location: location.clone(),
                last_modified: defaults.last_modified.clone(),
                change_frequency: defaults.change_frequency.clone(),
                priority: defaults.priority,
                ..SitemapEntry::default()
            });
            if let Some(last_modified) = &configured.last_modified {
                validate_last_modified(last_modified, &format!("{field}.lastModified"))?;
                entry.last_modified = Some(last_modified.clone());
            }
            if let Some(change_frequency) = &configured.change_frequency {
                entry.change_frequency = Some(change_frequency.clone());
            }
            if let Some(priority) = configured.priority {
                validate_priority(priority, &format!("{field}.priority"))?;
                entry.priority = Some(priority);
            }
            entry.alternates = normalize_alternates(&configured.alternates, &field)?;
            entry.images = configured
                .images
                .iter()
                .enumerate()
                .map(|(image_index, value)| {
                    normalize_absolute_http_url(value, &format!("{field}.images[{image_index}]"))
                })
                .collect::<std::io::Result<Vec<_>>>()?;
            entry.videos = configured.videos.clone();
            for (video_index, video) in entry.videos.iter_mut().enumerate() {
                normalize_video(video, &format!("{field}.videos[{video_index}]"))?;
            }
            entries.insert(location, entry);
        }
    }

    Ok(entries.into_values().collect())
}

fn sitemap_documents(entries: &[SitemapEntry]) -> std::io::Result<Vec<String>> {
    sitemap_documents_with_limits(entries, SITEMAP_MAX_URLS, SITEMAP_MAX_BYTES)
}

fn sitemap_documents_with_limits(
    entries: &[SitemapEntry],
    max_urls: usize,
    max_bytes: usize,
) -> std::io::Result<Vec<String>> {
    let features = sitemap_features(entries);
    let header = sitemap_header(features);
    if max_urls == 0 || max_bytes <= header.len() + SITEMAP_FOOTER.len() {
        return Err(invalid_input("sitemap limits must allow at least one URL"));
    }

    let mut documents = Vec::new();
    let mut document = header.clone();
    let mut url_count = 0usize;

    for sitemap_entry in entries {
        let entry = sitemap_entry_xml(sitemap_entry);
        let exceeds_bytes = document.len() + entry.len() + SITEMAP_FOOTER.len() > max_bytes;
        if url_count > 0 && (url_count == max_urls || exceeds_bytes) {
            document.push_str(SITEMAP_FOOTER);
            documents.push(document);
            document = header.clone();
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

fn sitemap_features(entries: &[SitemapEntry]) -> SitemapFeatures {
    SitemapFeatures {
        alternates: entries.iter().any(|entry| !entry.alternates.is_empty()),
        images: entries.iter().any(|entry| !entry.images.is_empty()),
        videos: entries.iter().any(|entry| !entry.videos.is_empty()),
    }
}

fn sitemap_header(features: SitemapFeatures) -> String {
    let mut header = String::from(SITEMAP_XML_DECLARATION);
    header.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\"");
    if features.alternates {
        header.push_str(" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\"");
    }
    if features.images {
        header.push_str(" xmlns:image=\"http://www.google.com/schemas/sitemap-image/1.1\"");
    }
    if features.videos {
        header.push_str(" xmlns:video=\"http://www.google.com/schemas/sitemap-video/1.1\"");
    }
    header.push_str(">\n");
    header
}

fn sitemap_entry_xml(entry: &SitemapEntry) -> String {
    let mut xml = String::from("  <url>\n");
    push_xml_element(&mut xml, 4, "loc", &entry.location);
    for (language, href) in &entry.alternates {
        xml.push_str(&format!(
            "    <xhtml:link rel=\"alternate\" hreflang=\"{}\" href=\"{}\" />\n",
            escape_xml(language),
            escape_xml(href)
        ));
    }
    for image in &entry.images {
        xml.push_str("    <image:image>\n");
        push_xml_element(&mut xml, 6, "image:loc", image);
        xml.push_str("    </image:image>\n");
    }
    for video in &entry.videos {
        xml.push_str("    <video:video>\n");
        push_xml_element(&mut xml, 6, "video:title", &video.title);
        push_xml_element(&mut xml, 6, "video:thumbnail_loc", &video.thumbnail_loc);
        push_xml_element(&mut xml, 6, "video:description", &video.description);
        push_optional_xml_element(
            &mut xml,
            6,
            "video:content_loc",
            video.content_loc.as_deref(),
        );
        push_optional_xml_element(&mut xml, 6, "video:player_loc", video.player_loc.as_deref());
        push_optional_xml_element(
            &mut xml,
            6,
            "video:duration",
            video.duration.map(|value| value.to_string()).as_deref(),
        );
        push_optional_xml_element(
            &mut xml,
            6,
            "video:view_count",
            video.view_count.map(|value| value.to_string()).as_deref(),
        );
        push_optional_xml_element(
            &mut xml,
            6,
            "video:rating",
            video.rating.map(format_number).as_deref(),
        );
        push_optional_xml_element(
            &mut xml,
            6,
            "video:expiration_date",
            video.expiration_date.as_deref(),
        );
        push_optional_xml_element(
            &mut xml,
            6,
            "video:publication_date",
            video.publication_date.as_deref(),
        );
        push_optional_xml_element(
            &mut xml,
            6,
            "video:family_friendly",
            video.family_friendly.as_ref().map(YesNo::as_str),
        );
        push_optional_xml_element(
            &mut xml,
            6,
            "video:requires_subscription",
            video.requires_subscription.as_ref().map(YesNo::as_str),
        );
        push_optional_xml_element(
            &mut xml,
            6,
            "video:live",
            video.live.as_ref().map(YesNo::as_str),
        );
        push_video_relationship(&mut xml, "restriction", video.restriction.as_ref());
        push_video_relationship(&mut xml, "platform", video.platform.as_ref());
        if let Some(uploader) = &video.uploader {
            let info = uploader.info.as_ref().map_or_else(String::new, |value| {
                format!(" info=\"{}\"", escape_xml(value))
            });
            xml.push_str(&format!(
                "      <video:uploader{info}>{}</video:uploader>\n",
                escape_xml(&uploader.content)
            ));
        }
        for tag in video
            .tag
            .as_ref()
            .map(OneOrManyStrings::values)
            .unwrap_or_default()
        {
            push_xml_element(&mut xml, 6, "video:tag", tag);
        }
        xml.push_str("    </video:video>\n");
    }
    push_optional_xml_element(&mut xml, 4, "lastmod", entry.last_modified.as_deref());
    push_optional_xml_element(
        &mut xml,
        4,
        "changefreq",
        entry
            .change_frequency
            .as_ref()
            .map(SitemapChangeFrequency::as_str),
    );
    push_optional_xml_element(
        &mut xml,
        4,
        "priority",
        entry.priority.map(format_number).as_deref(),
    );
    xml.push_str("  </url>\n");
    xml
}

fn push_xml_element(xml: &mut String, spaces: usize, name: &str, value: &str) {
    xml.push_str(&format!(
        "{}{name_open}{}{name_close}\n",
        " ".repeat(spaces),
        escape_xml(value),
        name_open = format_args!("<{name}>"),
        name_close = format_args!("</{name}>")
    ));
}

fn push_optional_xml_element(xml: &mut String, spaces: usize, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        push_xml_element(xml, spaces, name, value);
    }
}

fn push_video_relationship(xml: &mut String, name: &str, value: Option<&SitemapVideoRelationship>) {
    if let Some(value) = value {
        xml.push_str(&format!(
            "      <video:{name} relationship=\"{}\">{}</video:{name}>\n",
            value.relationship.as_str(),
            escape_xml(&value.content)
        ));
    }
}

fn format_number(value: f64) -> String {
    value.to_string()
}

fn validate_entry_metadata(value: &SitemapEntryMetadata, field: &str) -> std::io::Result<()> {
    if let Some(last_modified) = &value.last_modified {
        validate_last_modified(last_modified, &format!("{field}.lastModified"))?;
    }
    if let Some(priority) = value.priority {
        validate_priority(priority, &format!("{field}.priority"))?;
    }
    Ok(())
}

fn validate_last_modified(value: &str, field: &str) -> std::io::Result<()> {
    let valid = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
        || chrono::DateTime::parse_from_rfc3339(value).is_ok();
    if valid {
        Ok(())
    } else {
        Err(invalid_input(format!(
            "{field} must be an ISO 8601 date or RFC 3339 timestamp"
        )))
    }
}

fn validate_priority(value: f64, field: &str) -> std::io::Result<()> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(invalid_input(format!("{field} must be between 0 and 1")))
    }
}

fn validate_sitemap_location(value: &str, field: &str) -> std::io::Result<()> {
    if value.chars().count() > SITEMAP_MAX_LOCATION_CHARS {
        return Err(invalid_input(format!(
            "{field} URL exceeds {SITEMAP_MAX_LOCATION_CHARS} characters: {value}"
        )));
    }
    Ok(())
}

fn normalize_sitemap_entry_url(
    value: &str,
    site_url: &str,
    field: &str,
) -> std::io::Result<String> {
    let location = if value.starts_with('/') {
        validate_application_path(value, &format!("{field}.url"))?;
        format!("{site_url}{}", percent_encode_path(value))
    } else {
        let normalized = normalize_absolute_http_url(value, &format!("{field}.url"))?;
        if normalized == site_url {
            format!("{site_url}/")
        } else if normalized.starts_with(&format!("{site_url}/"))
            || normalized.starts_with(&format!("{site_url}?"))
        {
            normalized
        } else {
            return Err(invalid_input(format!(
                "{field}.url must use the configured sitemap origin {site_url}"
            )));
        }
    };
    validate_sitemap_location(&location, field)?;
    Ok(location)
}

fn normalize_alternates(
    alternates: &SitemapAlternates,
    field: &str,
) -> std::io::Result<BTreeMap<String, String>> {
    alternates
        .languages
        .iter()
        .map(|(language, href)| {
            if language.is_empty()
                || !language
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            {
                return Err(invalid_input(format!(
                    "{field}.alternates.languages contains invalid language tag {language}"
                )));
            }
            Ok((
                language.clone(),
                normalize_absolute_http_url(
                    href,
                    &format!("{field}.alternates.languages.{language}"),
                )?,
            ))
        })
        .collect()
}

fn normalize_video(video: &mut SitemapVideo, field: &str) -> std::io::Result<()> {
    validate_non_empty_text(&video.title, &format!("{field}.title"))?;
    validate_non_empty_text(&video.description, &format!("{field}.description"))?;
    video.thumbnail_loc =
        normalize_absolute_http_url(&video.thumbnail_loc, &format!("{field}.thumbnail_loc"))?;
    if let Some(value) = video.content_loc.as_mut() {
        *value = normalize_absolute_http_url(value, &format!("{field}.content_loc"))?;
    }
    if let Some(value) = video.player_loc.as_mut() {
        *value = normalize_absolute_http_url(value, &format!("{field}.player_loc"))?;
    }
    if let Some(duration) = video.duration
        && !(1..=28_800).contains(&duration)
    {
        return Err(invalid_input(format!(
            "{field}.duration must be between 1 and 28800 seconds"
        )));
    }
    if let Some(rating) = video.rating
        && (!rating.is_finite() || !(0.0..=5.0).contains(&rating))
    {
        return Err(invalid_input(format!(
            "{field}.rating must be between 0 and 5"
        )));
    }
    for (name, value) in [
        ("expiration_date", video.expiration_date.as_deref()),
        ("publication_date", video.publication_date.as_deref()),
    ] {
        if let Some(value) = value {
            validate_last_modified(value, &format!("{field}.{name}"))?;
        }
    }
    for (name, relationship) in [
        ("restriction", video.restriction.as_ref()),
        ("platform", video.platform.as_ref()),
    ] {
        if let Some(relationship) = relationship {
            validate_non_empty_text(&relationship.content, &format!("{field}.{name}.content"))?;
        }
    }
    if let Some(uploader) = &mut video.uploader {
        validate_non_empty_text(&uploader.content, &format!("{field}.uploader.content"))?;
        if let Some(info) = &mut uploader.info {
            *info = normalize_absolute_http_url(info, &format!("{field}.uploader.info"))?;
        }
    }
    for tag in video
        .tag
        .as_ref()
        .map(OneOrManyStrings::values)
        .unwrap_or_default()
    {
        validate_non_empty_text(tag, &format!("{field}.tag"))?;
    }
    Ok(())
}

fn validate_non_empty_text(value: &str, field: &str) -> std::io::Result<()> {
    if value.trim().is_empty() {
        return Err(invalid_input(format!("{field} must not be empty")));
    }
    validate_single_line(value, field)
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
            i18n: None,
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
            ..SitemapGenerationOptions::default()
        };
        let entries = sitemap_entries(
            &manifest,
            &["/en/docs/routing & metadata".to_string(), "/".to_string()],
            "https://ruvyxa.dev",
            Some(&setting),
        )
        .unwrap();
        let xml = &sitemap_documents(&entries).unwrap()[0];

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
        let entries = sitemap_entries(&manifest, &[], "https://ruvyxa.dev", None).unwrap();
        let xml = &sitemap_documents(&entries).unwrap()[0];
        assert!(xml.contains("<loc>https://ruvyxa.dev/</loc>"), "{xml}");
        assert!(xml.contains("<loc>https://ruvyxa.dev/blog</loc>"), "{xml}");
        assert!(!xml.contains("[slug]"), "{xml}");
        assert!(!xml.contains("/api/health"), "{xml}");
    }

    #[test]
    fn sitemap_shards_by_protocol_limits_and_writes_an_index() {
        let entries = ["/a", "/b", "/c"]
            .into_iter()
            .map(|path| SitemapEntry {
                location: format!("https://ruvyxa.dev{path}"),
                ..SitemapEntry::default()
            })
            .collect::<Vec<_>>();
        let documents = sitemap_documents_with_limits(&entries, 2, 10_000).unwrap();
        assert_eq!(documents.len(), 2);
        assert_eq!(documents[0].matches("<url>").count(), 2);
        assert_eq!(documents[1].matches("<url>").count(), 1);
        let index = sitemap_index_xml("https://ruvyxa.dev", documents.len()).unwrap();
        assert!(index.contains("https://ruvyxa.dev/sitemap-0.xml"));
        assert!(index.contains("https://ruvyxa.dev/sitemap-1.xml"));
    }

    #[test]
    fn sitemap_renders_next_style_metadata_and_extension_namespaces() {
        let options: SitemapGenerationOptions = serde_json::from_value(serde_json::json!({
            "defaults": {
                "lastModified": "2026-07-29",
                "changeFrequency": "weekly",
                "priority": 0.5
            },
            "entries": [{
                "url": "/about",
                "lastModified": "2026-07-29T04:30:00.000Z",
                "changeFrequency": "monthly",
                "priority": 0.8,
                "alternates": {
                    "languages": {
                        "de": "https://ruvyxa.dev/de/about",
                        "th": "https://ruvyxa.dev/th/about"
                    }
                },
                "images": ["https://cdn.ruvyxa.dev/about.jpg"],
                "videos": [{
                    "title": "Ruvyxa & Next-style XML",
                    "thumbnail_loc": "https://cdn.ruvyxa.dev/thumb.jpg",
                    "description": "A <production> sitemap example",
                    "content_loc": "https://cdn.ruvyxa.dev/video.mp4",
                    "duration": 120,
                    "rating": 4.5,
                    "family_friendly": "yes",
                    "restriction": { "relationship": "allow", "content": "TH US" },
                    "tag": ["framework", "sitemap"]
                }]
            }]
        }))
        .unwrap();
        let entries = sitemap_entries(
            &manifest(&[("/", RouteKind::Page), ("/about", RouteKind::Page)]),
            &[],
            "https://ruvyxa.dev",
            Some(&options),
        )
        .unwrap();
        let xml = &sitemap_documents(&entries).unwrap()[0];

        assert!(xml.contains("xmlns:xhtml=\"http://www.w3.org/1999/xhtml\""));
        assert!(xml.contains("xmlns:image=\"http://www.google.com/schemas/sitemap-image/1.1\""));
        assert!(xml.contains("xmlns:video=\"http://www.google.com/schemas/sitemap-video/1.1\""));
        assert!(xml.contains("  <url>\n    <loc>https://ruvyxa.dev/about</loc>"));
        assert!(xml.contains("<lastmod>2026-07-29T04:30:00.000Z</lastmod>"));
        assert!(xml.contains("<changefreq>monthly</changefreq>"));
        assert!(xml.contains("<priority>0.8</priority>"));
        assert!(xml.contains("hreflang=\"th\" href=\"https://ruvyxa.dev/th/about\""));
        assert!(xml.contains("<image:loc>https://cdn.ruvyxa.dev/about.jpg</image:loc>"));
        assert!(xml.contains("<video:title>Ruvyxa &amp; Next-style XML</video:title>"));
        assert!(xml.contains(
            "<video:description>A &lt;production&gt; sitemap example</video:description>"
        ));
        assert_eq!(xml.matches("<url>").count(), 2);
    }

    #[test]
    fn sitemap_rejects_invalid_rich_entry_values() {
        for value in [
            serde_json::json!({ "entries": [{ "url": "https://evil.example/about" }] }),
            serde_json::json!({ "entries": [{ "url": "/about", "priority": 1.1 }] }),
            serde_json::json!({ "entries": [{ "url": "/about", "lastModified": "yesterday" }] }),
            serde_json::json!({
                "entries": [{
                    "url": "/about",
                    "videos": [{
                        "title": "video",
                        "thumbnail_loc": "javascript:alert(1)",
                        "description": "bad URL"
                    }]
                }]
            }),
        ] {
            let options: SitemapGenerationOptions = serde_json::from_value(value).unwrap();
            assert!(
                sitemap_entries(
                    &manifest(&[("/about", RouteKind::Page)]),
                    &[],
                    "https://ruvyxa.dev",
                    Some(&options)
                )
                .is_err()
            );
        }
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
                ..SiteConfigOptions::default()
            },
        )
        .unwrap();
        assert_eq!(report, DiscoveryReport::default());
        assert!(!assets.join("robots.txt").exists());
        assert!(!assets.join("sitemap.xml").exists());
    }
}
