//! Response construction and the security headers every Ruvyxa response carries.
//!
//! ## One list of headers
//!
//! The defaults are declared once, in [`DEFAULT_SECURITY_HEADERS`], and both
//! directions read it: [`apply_security_headers`] inserts what is missing and
//! [`finalize_security_headers`] removes what it inserted when a project turns
//! the feature off. They used to be two hand-written sequences of the same seven
//! headers, so adding one meant remembering to add it twice — and forgetting the
//! removal half is silent: `security: false` would keep sending a header the
//! project asked not to send, which nothing tests for and no error reports.
//!
//! The same seven headers are also served by the JavaScript runtimes, from
//! `DEFAULT_SECURITY_HEADERS` in `packages/@ruvyxa/core/src/utils.ts`, which
//! generates the `_headers` file hosts read. That copy cannot import this one,
//! so `tests/fixtures/security-headers-conformance.json` holds both to the same
//! list: a header added to one language and not the other means the same site
//! is protected differently depending on where it is deployed.

use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use serde::Serialize;

/// Response headers Ruvyxa sends unless the application sets its own.
///
/// Names are lowercase because `HeaderName::from_static` requires it.
pub(crate) const DEFAULT_SECURITY_HEADERS: [(&str, &str); 7] = [
    ("x-content-type-options", "nosniff"),
    ("referrer-policy", "strict-origin-when-cross-origin"),
    (
        "permissions-policy",
        "camera=(), microphone=(), geolocation=()",
    ),
    ("cross-origin-opener-policy", "same-origin"),
    ("cross-origin-resource-policy", "same-origin"),
    ("x-frame-options", "DENY"),
    ("x-permitted-cross-domain-policies", "none"),
];

/// Add every default header the response does not already set.
///
/// An application header always wins: these are defaults, not a policy imposed
/// over what a route deliberately chose.
pub(crate) fn apply_security_headers(response: &mut Response) {
    let headers = response.headers_mut();
    for (name, value) in DEFAULT_SECURITY_HEADERS {
        insert_default_header(
            headers,
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        );
    }
}

fn insert_default_header(headers: &mut HeaderMap, name: HeaderName, value: HeaderValue) {
    if !headers.contains_key(&name) {
        headers.insert(name, value);
    }
}

/// Apply or strip the defaults according to the project's `security` setting.
///
/// Stripping only removes a header that still holds the exact default value, so
/// a header an application set deliberately survives either way.
pub(crate) fn finalize_security_headers(mut response: Response, enabled: bool) -> Response {
    if enabled {
        apply_security_headers(&mut response);
        return response;
    }
    let headers = response.headers_mut();
    for (name, value) in DEFAULT_SECURITY_HEADERS {
        remove_default_header(headers, HeaderName::from_static(name), value);
    }
    response
}

fn remove_default_header(headers: &mut HeaderMap, name: HeaderName, default_value: &str) {
    if headers
        .get(&name)
        .is_some_and(|value| value.as_bytes() == default_value.as_bytes())
    {
        headers.remove(name);
    }
}

pub(crate) fn with_security_headers(mut response: Response) -> Response {
    apply_security_headers(&mut response);
    response
}

pub(crate) fn html_response(status: StatusCode, body: String) -> Response {
    html_response_from_body(status, Body::from(body))
}

/// Serve an HTML document that is already stored behind an [`Arc<str>`].
///
/// The render cache hands out shared allocations, so a cache hit can build the
/// response body without copying the document. Building it from a `String`
/// instead meant one full copy of every cached page on every hit.
pub(crate) fn shared_html_response(status: StatusCode, body: Arc<str>) -> Response {
    html_response_from_body(status, shared_text_body(body))
}

/// Lets `Bytes` borrow an `Arc<str>` as its backing storage.
struct SharedText(Arc<str>);

impl AsRef<[u8]> for SharedText {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

/// Build a response body from a shared string without copying it.
pub(crate) fn shared_text_body(text: Arc<str>) -> Body {
    Body::from(Bytes::from_owner(SharedText(text)))
}

fn html_response_from_body(status: StatusCode, body: Body) -> Response {
    let mut response = (status, Html(body)).into_response();
    if status.is_client_error() || status.is_server_error() {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, max-age=0"),
        );
    }
    apply_security_headers(&mut response);
    response
}

pub(crate) fn json_response<T: Serialize>(status: StatusCode, value: &T) -> Response {
    match serde_json::to_string(value) {
        Ok(body) => {
            let mut response = (status, body).into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            );
            apply_security_headers(&mut response);
            response
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize JSON response: {error}"),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain_response() -> Response {
        StatusCode::OK.into_response()
    }

    /// Whatever `apply` adds, `finalize(.., false)` has to be able to take back
    /// off. Two hand-maintained lists could not promise that; one list does.
    #[test]
    fn disabling_security_removes_exactly_what_enabling_adds() {
        let mut response = plain_response();
        apply_security_headers(&mut response);
        assert_eq!(
            response.headers().len(),
            DEFAULT_SECURITY_HEADERS.len(),
            "every default must be applied"
        );

        let stripped = finalize_security_headers(response, false);
        assert!(
            stripped.headers().is_empty(),
            "disabling security must leave no default behind: {:?}",
            stripped.headers()
        );
    }

    /// Defaults, not overrides: a route that set a header keeps its value, and
    /// disabling security must not delete it either.
    #[test]
    fn an_application_header_survives_both_directions() {
        let mut response = plain_response();
        response.headers_mut().insert(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("SAMEORIGIN"),
        );

        apply_security_headers(&mut response);
        assert_eq!(response.headers()["x-frame-options"], "SAMEORIGIN");

        let stripped = finalize_security_headers(response, false);
        assert_eq!(
            stripped.headers()["x-frame-options"],
            "SAMEORIGIN",
            "a deliberate application header is not a Ruvyxa default to strip"
        );
    }

    /// Held to the same list the JavaScript runtimes serve from
    /// `@ruvyxa/core/utils`, which no Rust code can import.
    #[test]
    fn matches_the_shared_cross_language_security_header_list() {
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/security-headers-conformance.json");
        let fixture: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&fixture_path)
                .unwrap_or_else(|error| panic!("read {}: {error}", fixture_path.display())),
        )
        .expect("conformance fixture is valid JSON");

        let declared = fixture["headers"]
            .as_object()
            .expect("fixture declares headers");
        let actual: std::collections::BTreeMap<String, String> = DEFAULT_SECURITY_HEADERS
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect();
        let expected: std::collections::BTreeMap<String, String> = declared
            .iter()
            .map(|(name, value)| {
                (
                    name.to_ascii_lowercase(),
                    value
                        .as_str()
                        .expect("header value is a string")
                        .to_string(),
                )
            })
            .collect();

        assert_eq!(
            actual, expected,
            "the fixture decides the list; JavaScript replays the same file"
        );
    }

    /// An error page must not be cached, or a transient 500 sticks in a shared
    /// cache long after the deploy that caused it.
    #[test]
    fn error_documents_are_not_cacheable() {
        let response = html_response(StatusCode::INTERNAL_SERVER_ERROR, "boom".into());
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "no-store, max-age=0"
        );

        let ok = html_response(StatusCode::OK, "fine".into());
        assert!(!ok.headers().contains_key(header::CACHE_CONTROL));
    }
}
