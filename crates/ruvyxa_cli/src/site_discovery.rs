//! Default `robots.txt` and `sitemap.xml` generation.
//!
//! Both files are what a crawler asks for before it asks for anything else, and
//! a site that answers them with an HTML page — which is what a bare dynamic
//! route such as `/[lang]` does when no file exists — fails the corresponding
//! Lighthouse SEO audits. The build therefore emits them from the route
//! manifest by default rather than leaving it to an opt-in plugin.
//!
//! Two rules keep this from surprising anyone:
//!
//! - A file the project ships in `public/` always wins. Asset preparation has
//!   already copied `public/` into the staging assets directory by the time this
//!   runs, so an existing file is left untouched.
//! - `sitemap.xml` needs absolute URLs, so it is only written when a site URL is
//!   known. `robots.txt` is valid without one and is always written.

use std::fs;
use std::path::Path;

use ruvyxa_graph::{RouteKind, RouteManifest};

/// `site` block of `ruvyxa.config.ts`.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SiteConfigOptions {
    /// Absolute origin of the deployed site, e.g. `https://ruvyxa.dev`.
    pub url: Option<String>,
    /// Emit `sitemap.xml`. Requires a resolvable site URL. @default true
    pub sitemap: Option<bool>,
    /// Emit `robots.txt`. @default true
    pub robots: Option<bool>,
}

impl SiteConfigOptions {
    fn sitemap_enabled(&self) -> bool {
        self.sitemap.unwrap_or(true)
    }

    fn robots_enabled(&self) -> bool {
        self.robots.unwrap_or(true)
    }
}

/// What the build wrote, for the CLI summary and for `check` to report on.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DiscoveryReport {
    pub robots_written: bool,
    pub sitemap_written: bool,
    /// Set when a sitemap was wanted but no site URL could be resolved.
    pub sitemap_needs_site_url: bool,
}

/// Environment variable Ruvyxa reads when the site URL is not configured.
///
/// One variable Ruvyxa owns, rather than a list of host-specific names: a
/// deploy pipeline that knows its own URL exports this, and the framework stays
/// out of the business of tracking what each host happens to call it.
const SITE_URL_ENV_VAR: &str = "RUVYXA_SITE_URL";

/// Resolve the site's absolute origin from config, then from the environment.
///
/// A bare hostname is accepted and given an `https` scheme, since that is how
/// deploy pipelines usually expose it. Returns the origin without a trailing
/// slash.
pub fn resolve_site_url<F>(configured: Option<&str>, env: F) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    let candidate = configured
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| env(SITE_URL_ENV_VAR).filter(|value| !value.trim().is_empty()))?;

    let trimmed = candidate.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let normalized = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    // A value that is not a URL at all would produce a sitemap full of
    // unresolvable locations, which is worse than shipping no sitemap.
    if normalized.starts_with("http://") || normalized.starts_with("https://") {
        Some(normalized)
    } else {
        None
    }
}

/// Write the discovery files that are missing from the staged assets.
///
/// `prerendered_paths` carries the concrete URLs the build produced for dynamic
/// routes. A pattern such as `/[lang]/docs/[section]/[slug]` is not a URL, so
/// without them a documentation site would advertise a sitemap containing only
/// its home page while 69 real pages went unlisted.
pub fn write_discovery_files(
    manifest: &RouteManifest,
    prerendered_paths: &[String],
    assets_dir: &Path,
    site_url: Option<&str>,
    options: &SiteConfigOptions,
) -> std::io::Result<DiscoveryReport> {
    let mut report = DiscoveryReport::default();

    let sitemap_path = assets_dir.join("sitemap.xml");
    let wants_sitemap = options.sitemap_enabled() && !sitemap_path.exists();
    if wants_sitemap {
        match site_url {
            Some(url) => {
                fs::create_dir_all(assets_dir)?;
                fs::write(&sitemap_path, sitemap_xml(manifest, prerendered_paths, url))?;
                report.sitemap_written = true;
            }
            None => report.sitemap_needs_site_url = true,
        }
    }

    let robots_path = assets_dir.join("robots.txt");
    if options.robots_enabled() && !robots_path.exists() {
        fs::create_dir_all(assets_dir)?;
        // Only advertise a sitemap that will actually be there.
        let sitemap_url = site_url.filter(|_| report.sitemap_written || sitemap_path.exists());
        fs::write(&robots_path, robots_txt(sitemap_url))?;
        report.robots_written = true;
    }

    Ok(report)
}

/// Indexable page URLs, sorted and deduplicated.
///
/// Static page routes are URLs as written. A dynamic pattern is not, so its
/// entries come from the paths the build actually prerendered; a dynamic route
/// with no prerendered output contributes nothing, which is correct — there is
/// no URL to advertise until one exists.
fn indexable_paths<'a>(
    manifest: &'a RouteManifest,
    prerendered_paths: &'a [String],
) -> Vec<&'a str> {
    let mut paths: Vec<&str> = manifest
        .routes
        .iter()
        .filter(|route| route.kind == RouteKind::Page && !route.path.contains('['))
        .map(|route| route.path.as_str())
        .chain(
            prerendered_paths
                .iter()
                .map(String::as_str)
                .filter(|path| !path.contains('[')),
        )
        .collect();
    paths.sort_unstable();
    paths.dedup();
    paths
}

fn sitemap_xml(manifest: &RouteManifest, prerendered_paths: &[String], site_url: &str) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for path in indexable_paths(manifest, prerendered_paths) {
        // The site URL has no trailing slash and every route path starts with
        // one, except the root, which must not become `https://site.dev`.
        let location = if path == "/" {
            format!("{site_url}/")
        } else {
            format!("{site_url}{path}")
        };
        xml.push_str("  <url><loc>");
        xml.push_str(&escape_xml(&location));
        xml.push_str("</loc></url>\n");
    }
    xml.push_str("</urlset>\n");
    xml
}

fn robots_txt(sitemap_url: Option<&str>) -> String {
    let mut text = String::from("User-agent: *\nAllow: /\n");
    if let Some(site_url) = sitemap_url {
        text.push_str(&format!("\nSitemap: {site_url}/sitemap.xml\n"));
    }
    text
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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
    fn site_url_prefers_config_then_host_environment() {
        assert_eq!(
            resolve_site_url(Some("https://ruvyxa.dev/"), |_| Some(
                "https://env".to_string()
            )),
            Some("https://ruvyxa.dev".to_string())
        );
        // A bare hostname from a deploy pipeline is given a scheme.
        assert_eq!(
            resolve_site_url(None, |name| (name == "RUVYXA_SITE_URL")
                .then(|| "demo.ruvyxa.dev".to_string())),
            Some("https://demo.ruvyxa.dev".to_string())
        );
        // No other environment variable is consulted.
        assert_eq!(
            resolve_site_url(None, |name| (name != "RUVYXA_SITE_URL")
                .then(|| "https://wrong.example".to_string())),
            None
        );
        assert_eq!(resolve_site_url(None, no_env), None);
        assert_eq!(resolve_site_url(Some("   "), no_env), None);
        // A value that is not a URL would fill the sitemap with unresolvable
        // locations; no sitemap is better than a broken one.
        assert_eq!(resolve_site_url(Some("ftp://ruvyxa.dev"), no_env), None);
    }

    #[test]
    fn sitemap_lists_prerendered_urls_for_dynamic_routes() {
        // A locale-segmented documentation site is entirely dynamic patterns;
        // listing only the static routes would advertise one URL for a site
        // that actually publishes dozens.
        let manifest = manifest(&[
            ("/", RouteKind::Page),
            ("/[lang]", RouteKind::Page),
            ("/[lang]/docs/[slug]", RouteKind::Page),
        ]);
        let prerendered = [
            "/en".to_string(),
            "/th".to_string(),
            "/en/docs/routing".to_string(),
            // A duplicate of a static route must not be listed twice.
            "/".to_string(),
        ];

        let xml = sitemap_xml(&manifest, &prerendered, "https://ruvyxa.dev");

        assert_eq!(xml.matches("<loc>").count(), 4, "{xml}");
        assert!(
            xml.contains("<loc>https://ruvyxa.dev/en/docs/routing</loc>"),
            "{xml}"
        );
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

        let xml = sitemap_xml(&manifest, &[], "https://ruvyxa.dev");

        assert!(xml.contains("<loc>https://ruvyxa.dev/</loc>"), "{xml}");
        assert!(xml.contains("<loc>https://ruvyxa.dev/blog</loc>"), "{xml}");
        // A route pattern is not a URL, and an API route is not a page.
        assert!(!xml.contains("[slug]"), "{xml}");
        assert!(!xml.contains("/api/health"), "{xml}");
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
        // The build could not produce a sitemap, so robots must not point at one.
        assert!(report.sitemap_needs_site_url);
        let robots = fs::read_to_string(assets.join("robots.txt")).unwrap();
        assert_eq!(robots, "User-agent: *\nAllow: /\n");
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
        assert!(!report.sitemap_needs_site_url);
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
                sitemap: Some(false),
                robots: Some(false),
            },
        )
        .unwrap();

        assert_eq!(report, DiscoveryReport::default());
        assert!(!assets.join("robots.txt").exists());
        assert!(!assets.join("sitemap.xml").exists());
    }
}
