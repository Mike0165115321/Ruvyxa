//! Head elements contributed by plugins.
//!
//! A plugin declares these once in `ruvyxa.config.ts`; the server renders them
//! into every document's `<head>` alongside the asset links and collected CSS.
//! Injecting them here rather than through a plugin hook keeps the render path
//! free of a per-request round trip into the plugin host, which is what made
//! analytics and verification-tag plugins impractical to write before.
//!
//! Per-route metadata is a different problem and belongs to the route's `meta`
//! export: a plugin declaration cannot know which route is rendering.

use serde::Deserialize;

use crate::html_document::escape_html;

/// Elements legal inside `<head>`.
///
/// Closed on purpose: an unexpected element there ends the head early and the
/// browser moves the rest of the document into `<body>`. `definePlugin`
/// validates the same list, so an entry that reaches this point is already
/// well-formed; the check is repeated because the config file is data the
/// server reads, not code it trusts.
const ALLOWED_TAGS: [&str; 5] = ["link", "meta", "noscript", "script", "style"];

/// Elements whose content model is raw text.
const RAW_TEXT_TAGS: [&str; 3] = ["noscript", "script", "style"];

/// Void elements, which must not be given a closing tag.
const VOID_TAGS: [&str; 2] = ["link", "meta"];

#[derive(Debug, Clone, Deserialize)]
pub struct PluginHeadEntry {
    pub tag: String,
    #[serde(default)]
    pub attrs: std::collections::BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub children: Option<String>,
}

/// Render declared entries into a `<head>` fragment.
///
/// Invalid entries are skipped rather than failing the render: a malformed
/// plugin declaration should not take a production page down, and
/// `definePlugin` already rejects it at config load with a real error.
pub fn render_plugin_head(entries: &[PluginHeadEntry]) -> String {
    let mut html = String::new();
    for entry in entries {
        let tag = entry.tag.to_ascii_lowercase();
        if !ALLOWED_TAGS.contains(&tag.as_str()) {
            continue;
        }
        html.push('<');
        html.push_str(&tag);
        for (name, value) in &entry.attrs {
            if !is_safe_attribute_name(name) {
                continue;
            }
            let Some(value) = attribute_value(value) else {
                continue;
            };
            html.push(' ');
            html.push_str(name);
            html.push_str("=\"");
            html.push_str(&escape_html(&value));
            html.push('"');
        }
        html.push('>');

        if VOID_TAGS.contains(&tag.as_str()) {
            continue;
        }
        if let Some(children) = &entry.children
            && RAW_TEXT_TAGS.contains(&tag.as_str())
        {
            // Raw text is written verbatim — escaping would corrupt a script or
            // stylesheet. A nested close tag would end the element early, so it
            // is the one thing rejected here and at declaration time.
            if !contains_close_tag(children, &tag) {
                html.push_str(children);
            }
        }
        html.push_str("</");
        html.push_str(&tag);
        html.push('>');
    }
    html
}

/// An attribute name is written unescaped, so it must not be able to introduce
/// another attribute or close the tag.
fn is_safe_attribute_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|char| char.is_ascii_alphabetic())
        && chars.all(|char| char.is_ascii_alphanumeric() || matches!(char, ':' | '_' | '.' | '-'))
}

/// JSON scalars become attribute values; anything structured is dropped.
///
/// `true` renders as a bare-value attribute (`defer="true"`), which browsers
/// treat as present. `false` removes the attribute, matching JSX.
fn attribute_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Bool(true) => Some("true".to_string()),
        _ => None,
    }
}

fn contains_close_tag(text: &str, tag: &str) -> bool {
    let needle = format!("</{tag}");
    text.to_ascii_lowercase().contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        tag: &str,
        attrs: &[(&str, serde_json::Value)],
        children: Option<&str>,
    ) -> PluginHeadEntry {
        PluginHeadEntry {
            tag: tag.to_string(),
            attrs: attrs
                .iter()
                .map(|(name, value)| (name.to_string(), value.clone()))
                .collect(),
            children: children.map(str::to_string),
        }
    }

    #[test]
    fn renders_void_and_raw_text_elements() {
        let html = render_plugin_head(&[
            entry(
                "link",
                &[
                    ("rel", serde_json::json!("preconnect")),
                    ("href", serde_json::json!("https://cdn.example")),
                ],
                None,
            ),
            entry(
                "script",
                &[("async", serde_json::json!(true))],
                Some("console.log('hi')"),
            ),
        ]);

        // BTreeMap orders attributes by name, which keeps output deterministic.
        assert_eq!(
            html,
            "<link href=\"https://cdn.example\" rel=\"preconnect\">\
<script async=\"true\">console.log('hi')</script>"
        );
    }

    #[test]
    fn escapes_attribute_values_so_a_quote_cannot_close_the_tag() {
        let html = render_plugin_head(&[entry(
            "meta",
            &[
                ("name", serde_json::json!("description")),
                ("content", serde_json::json!("\"><script>alert(1)</script>")),
            ],
            None,
        )]);

        assert!(!html.contains("<script>"), "{html}");
        assert!(html.contains("&quot;&gt;&lt;script&gt;"), "{html}");
    }

    #[test]
    fn drops_entries_and_attributes_that_could_break_out_of_the_head() {
        let html = render_plugin_head(&[
            entry("div", &[], None),
            entry("meta", &[("onload=x y", serde_json::json!("1"))], None),
            entry("meta", &[("content", serde_json::json!({ "a": 1 }))], None),
            entry("style", &[], Some("a{}</style><script>alert(1)</script>")),
        ]);

        assert!(!html.contains("<div"), "{html}");
        assert!(!html.contains("onload"), "{html}");
        assert!(!html.contains("<script>"), "{html}");
        // The three surviving elements are emitted with no attributes or text.
        assert_eq!(html, "<meta><meta><style></style>");
    }

    #[test]
    fn omits_a_false_attribute_and_keeps_numbers() {
        let html = render_plugin_head(&[entry(
            "meta",
            &[
                ("charset", serde_json::json!(false)),
                ("content", serde_json::json!(30)),
            ],
            None,
        )]);
        assert_eq!(html, "<meta content=\"30\">");
    }

    #[test]
    fn a_void_element_never_gets_a_closing_tag() {
        let html = render_plugin_head(&[entry("link", &[], Some("ignored"))]);
        assert_eq!(html, "<link>");
    }
}
