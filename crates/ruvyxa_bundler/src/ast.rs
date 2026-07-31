//! Lightweight AST facts used by the Ruvyxa Bundler pipeline.
//!
//! This is intentionally smaller than a full JavaScript parser, but it gives
//! the resolver and transformer a shared structured view of imports, exports,
//! JSX, decorators, and TypeScript-only syntax instead of duplicating ad hoc
//! line scans in each stage.

use serde::{Deserialize, Serialize};

/// Import edge discovered in a source module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportEdge {
    pub specifier: String,
    pub kind: ImportKind,
}

/// The import form that created an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImportKind {
    Static,
    Dynamic,
    Require,
    ReExport,
    SideEffect,
}

/// Structured facts for one source module.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleAst {
    pub imports: Vec<ImportEdge>,
    pub exports: Vec<String>,
    pub has_jsx: bool,
    pub has_typescript: bool,
    pub has_decorators: bool,
    pub has_enums: bool,
}

impl ModuleAst {
    pub fn import_specifiers(&self) -> Vec<String> {
        self.imports
            .iter()
            .map(|edge| edge.specifier.clone())
            .collect()
    }

    pub fn dynamic_import_specifiers(&self) -> Vec<String> {
        self.imports
            .iter()
            .filter(|edge| edge.kind == ImportKind::Dynamic)
            .map(|edge| edge.specifier.clone())
            .collect()
    }
}

/// Parse source into the facts the bundler needs.
pub fn parse_module(source: &str) -> ModuleAst {
    let mut ast = ModuleAst::default();
    scan_code(source, 0, source.len(), &mut ast);
    ast
}

/// Scan `source[start..end]` as code, recording facts into `ast`.
///
/// Takes bounds rather than a substring so byte offsets stay absolute: the
/// scanner looks backwards (`is_line_prefix_whitespace`, `previous_non_whitespace`)
/// and a re-sliced string would make those reads consult the wrong bytes.
fn scan_code(source: &str, start: usize, end: usize, ast: &mut ModuleAst) {
    let bytes = &source.as_bytes()[..end];
    let mut index = start;
    // Last byte that can end a JavaScript token, so a `/` can be classified as
    // a regular expression or a division. See [`regex_can_start`].
    let mut previous_significant: Option<usize> = None;
    while index < bytes.len() {
        if is_comment_start(bytes, index) {
            index = skip_comment(bytes, index);
            continue;
        }
        if bytes[index] == b'`' {
            // A template literal's interpolations are code, not text. Skipping
            // the whole literal would hide `${require("server-only")}` from the
            // boundary check and drop real dependency edges.
            let (after, interpolations) = template_literal(bytes, index);
            for (code_start, code_end) in interpolations {
                scan_code(source, code_start, code_end, ast);
            }
            previous_significant = Some(index);
            index = after;
            continue;
        }
        if is_quote(bytes[index]) {
            let start = index;
            index = skip_string(bytes, index);
            previous_significant = Some(start);
            continue;
        }
        if bytes[index] == b'/' && regex_can_start(bytes, previous_significant) {
            let start = index;
            index = skip_regex_literal(bytes, index);
            previous_significant = Some(start);
            continue;
        }

        if bytes[index] == b'@' && is_line_prefix_whitespace(bytes, index) {
            ast.has_decorators = true;
            previous_significant = Some(index);
            index += 1;
            continue;
        }
        if bytes[index] == b'<' && looks_like_jsx_at(bytes, index) {
            ast.has_jsx = true;
        }

        if !is_ident_start_byte(bytes[index]) {
            if !bytes[index].is_ascii_whitespace() {
                previous_significant = Some(index);
            }
            index += 1;
            continue;
        }

        let start = index;
        index = skip_identifier(bytes, index);
        previous_significant = Some(index - 1);
        let word = &source[start..index];
        match word {
            "import" => {
                if let Some(edge) = import_edge(source, index, end) {
                    ast.imports.push(edge);
                }
            }
            "require" if previous_non_whitespace(bytes, start) != Some(b'.') => {
                if let Some(specifier) = call_specifier(source, index, end) {
                    ast.imports.push(ImportEdge {
                        specifier,
                        kind: ImportKind::Require,
                    });
                }
            }
            "export" => {
                if let Some(edge) = export_edge(source, index, end) {
                    ast.imports.push(edge);
                }
                if let Some(name) = export_name(source, index, end) {
                    ast.exports.push(name);
                }
            }
            "enum" => {
                ast.has_enums = true;
                ast.has_typescript = true;
            }
            "interface" | "type" | "satisfies" | "implements" | "declare" | "abstract"
            | "readonly" | "public" | "private" | "protected" | "override" => {
                ast.has_typescript = true;
            }
            "as" if previous_non_whitespace(bytes, start).is_some() => {
                ast.has_typescript = true;
            }
            _ => {}
        }
    }
}

fn import_edge(source: &str, after_keyword: usize, end: usize) -> Option<ImportEdge> {
    let bytes = &source.as_bytes()[..end];
    let index = skip_whitespace_and_comments(bytes, after_keyword);
    if index >= bytes.len() || bytes[index] == b'.' {
        return None;
    }
    if bytes[index] == b'(' {
        return call_specifier(source, index, end).map(|specifier| ImportEdge {
            specifier,
            kind: ImportKind::Dynamic,
        });
    }
    if is_quote(bytes[index]) {
        return quoted_value_at(source, index, end).map(|specifier| ImportEdge {
            specifier,
            kind: ImportKind::SideEffect,
        });
    }
    if word_at(source, index, end) == Some("type") {
        return None;
    }
    let declaration_start = index;
    find_from_specifier(source, declaration_start, end).map(|specifier| ImportEdge {
        specifier,
        kind: ImportKind::Static,
    })
}

fn export_edge(source: &str, after_keyword: usize, end: usize) -> Option<ImportEdge> {
    let bytes = &source.as_bytes()[..end];
    let index = skip_whitespace_and_comments(bytes, after_keyword);
    if word_at(source, index, end) == Some("type")
        || !matches!(bytes.get(index), Some(b'{') | Some(b'*'))
    {
        return None;
    }
    find_from_specifier(source, index, end).map(|specifier| ImportEdge {
        specifier,
        kind: ImportKind::ReExport,
    })
}

fn call_specifier(source: &str, after_keyword: usize, end: usize) -> Option<String> {
    let bytes = &source.as_bytes()[..end];
    let mut index = skip_whitespace_and_comments(bytes, after_keyword);
    if bytes.get(index) != Some(&b'(') {
        return None;
    }
    index = skip_whitespace_and_comments(bytes, index + 1);
    quoted_value_at(source, index, end)
}

fn find_from_specifier(source: &str, mut index: usize, end: usize) -> Option<String> {
    let bytes = &source.as_bytes()[..end];
    while index < bytes.len() {
        index = skip_whitespace_and_comments(bytes, index);
        if index >= bytes.len() || bytes[index] == b';' {
            return None;
        }
        if is_quote(bytes[index]) {
            index = skip_string(bytes, index);
            continue;
        }
        if word_at(source, index, end) == Some("from") {
            let value = skip_whitespace_and_comments(bytes, index + 4);
            return quoted_value_at(source, value, end);
        }
        index += 1;
    }
    None
}

fn quoted_value_at(source: &str, start: usize, end: usize) -> Option<String> {
    let bytes = &source.as_bytes()[..end];
    let quote = *bytes.get(start)?;
    if !is_quote(quote) || quote == b'`' {
        return None;
    }
    let mut index = start + 1;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index += 2;
            continue;
        }
        if bytes[index] == quote {
            return Some(source[start + 1..index].to_string());
        }
        index += 1;
    }
    None
}

/// Whether `source` declares a runtime default export.
///
/// Route validation needs this to tell a real page from a module that only
/// exports helpers. A plain `source.contains("export default")` answered the
/// question wrongly in both directions: it rejected `export { Page as default }`
/// and `export * as default from './page'`, which are valid default exports, and
/// it accepted a commented-out or quoted occurrence.
///
/// Reusing this module's comment- and string-skipping scanner means the answer
/// comes from the same view of the source the resolver and transformer already
/// share, rather than a third ad hoc text scan.
pub fn has_default_export(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut index = 0;
    let mut previous_significant: Option<usize> = None;
    while index < bytes.len() {
        if is_comment_start(bytes, index) {
            index = skip_comment(bytes, index);
            continue;
        }
        if bytes[index] == b'`' {
            // An `export` cannot appear inside an interpolation, but the
            // literal's extent still has to be measured the same way the
            // dependency scanner measures it, or the two disagree about where
            // code resumes.
            previous_significant = Some(index);
            index = template_literal(bytes, index).0;
            continue;
        }
        if is_quote(bytes[index]) {
            let start = index;
            index = skip_string(bytes, index);
            previous_significant = Some(start);
            continue;
        }
        if bytes[index] == b'/' && regex_can_start(bytes, previous_significant) {
            let start = index;
            index = skip_regex_literal(bytes, index);
            previous_significant = Some(start);
            continue;
        }
        if !is_ident_start_byte(bytes[index]) {
            if !bytes[index].is_ascii_whitespace() {
                previous_significant = Some(index);
            }
            index += 1;
            continue;
        }
        let start = index;
        index = skip_identifier(bytes, index);
        previous_significant = Some(index - 1);
        if &source[start..index] == "export" && export_declares_default(source, index, bytes.len())
        {
            return true;
        }
    }
    false
}

/// Whether the export clause starting after `export` produces a default binding.
fn export_declares_default(source: &str, after_keyword: usize, end: usize) -> bool {
    let bytes = &source.as_bytes()[..end];
    let index = skip_whitespace_and_comments(bytes, after_keyword);

    // `export type { Page as default }` and `export type default` are erased at
    // compile time and leave no runtime binding behind.
    if word_at(source, index, end) == Some("type") {
        return false;
    }

    match word_at(source, index, end) {
        // `export default …`
        Some("default") => true,
        Some(_) => false,
        None => match bytes.get(index) {
            // `export * as default from "./page"`
            Some(b'*') => {
                let index = skip_whitespace_and_comments(bytes, index + 1);
                if word_at(source, index, end) != Some("as") {
                    return false;
                }
                let index = skip_whitespace_and_comments(bytes, index + "as".len());
                word_at(source, index, end) == Some("default")
            }
            // `export { Page as default }`, `export { default } from "./page"`
            Some(b'{') => named_clause_exports_default(source, index, end),
            _ => false,
        },
    }
}

/// Whether a `{ … }` export clause binds something to the name `default`.
///
/// The exported name is the last identifier of each comma-separated specifier,
/// so `{ Page as default }` and `{ default }` both qualify while
/// `{ default as Page }` re-exports another module's default under a new name
/// and deliberately does not.
fn named_clause_exports_default(source: &str, brace: usize, end: usize) -> bool {
    let bytes = &source.as_bytes()[..end];
    let mut index = brace + 1;
    let mut last_word: Option<&str> = None;
    while index < bytes.len() {
        if is_comment_start(bytes, index) {
            index = skip_comment(bytes, index);
            continue;
        }
        if is_quote(bytes[index]) {
            // `export { "a-b" as default }` uses a string specifier name.
            index = skip_string(bytes, index);
            last_word = None;
            continue;
        }
        match bytes[index] {
            b'}' => return last_word == Some("default"),
            b',' => {
                if last_word == Some("default") {
                    return true;
                }
                last_word = None;
                index += 1;
            }
            byte if is_ident_start_byte(byte) => {
                let start = index;
                index = skip_identifier(bytes, index);
                let word = &source[start..index];
                // `as` is the separator, never the exported name.
                if word != "as" {
                    last_word = Some(word);
                }
            }
            _ => index += 1,
        }
    }
    false
}

fn export_name(source: &str, after_keyword: usize, end: usize) -> Option<String> {
    let bytes = &source.as_bytes()[..end];
    let mut index = skip_whitespace_and_comments(bytes, after_keyword);
    for optional in ["default", "async"] {
        if word_at(source, index, end) == Some(optional) {
            index = skip_whitespace_and_comments(bytes, index + optional.len());
        }
    }
    let kind = word_at(source, index, end)?;
    if !matches!(kind, "function" | "class" | "const" | "let" | "var") {
        return None;
    }
    index = skip_whitespace_and_comments(bytes, index + kind.len());
    if bytes.get(index) == Some(&b'*') {
        index = skip_whitespace_and_comments(bytes, index + 1);
    }
    let end = skip_identifier(bytes, index);
    (end > index).then(|| source[index..end].to_string())
}

fn word_at(source: &str, start: usize, end: usize) -> Option<&str> {
    let bytes = &source.as_bytes()[..end];
    if start >= bytes.len() || !is_ident_start_byte(bytes[start]) {
        return None;
    }
    Some(&source[start..skip_identifier(bytes, start)])
}

fn skip_whitespace_and_comments(bytes: &[u8], mut index: usize) -> usize {
    loop {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if is_comment_start(bytes, index) {
            index = skip_comment(bytes, index);
        } else {
            return index;
        }
    }
}

fn is_comment_start(bytes: &[u8], index: usize) -> bool {
    bytes.get(index) == Some(&b'/') && matches!(bytes.get(index + 1), Some(b'/') | Some(b'*'))
}

fn skip_comment(bytes: &[u8], start: usize) -> usize {
    if bytes.get(start + 1) == Some(&b'/') {
        return bytes[start + 2..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(bytes.len(), |offset| start + 2 + offset + 1);
    }
    let mut index = start + 2;
    while index + 1 < bytes.len() {
        if bytes[index] == b'*' && bytes[index + 1] == b'/' {
            return index + 2;
        }
        index += 1;
    }
    bytes.len()
}

/// Decide whether a `/` opens a regular expression rather than a division.
///
/// Every byte scanner in this crate needs this decision, and getting it wrong is
/// not a cosmetic error: without it `/["']/` reads as a division followed by an
/// unterminated string, and the string skip then swallows the rest of the module.
/// Imports after that point vanish from the dependency graph, `server-only`
/// markers stop being seen by the boundary check, and a page's default export
/// becomes invisible. Sharing one implementation is what keeps the scanners from
/// drifting back into that failure one at a time.
///
/// A regex may only appear where a value is expected. When the preceding token
/// could end a value (identifier, number, string, closing bracket) the slash is
/// division. Keywords such as `return` are values-expected positions.
///
/// `previous_significant` is the index of the last byte that can end a token, or
/// `None` at the start of the source.
pub(crate) fn regex_can_start(bytes: &[u8], previous_significant: Option<usize>) -> bool {
    let Some(index) = previous_significant else {
        return true;
    };
    match bytes[index] {
        b')' | b']' | b'}' | b'\'' | b'"' | b'`' => false,
        byte if is_ident_continue_byte(byte) => previous_token_is_keyword(bytes, index),
        _ => true,
    }
}

fn previous_token_is_keyword(bytes: &[u8], end: usize) -> bool {
    let mut start = end + 1;
    while start > 0 && is_ident_continue_byte(bytes[start - 1]) {
        start -= 1;
    }
    matches!(
        std::str::from_utf8(&bytes[start..=end]).unwrap_or_default(),
        "await"
            | "case"
            | "delete"
            | "do"
            | "else"
            | "in"
            | "instanceof"
            | "new"
            | "of"
            | "return"
            | "throw"
            | "typeof"
            | "void"
            | "yield"
    )
}

/// Skip past a regular expression literal, returning the index after it.
///
/// Quotes and slashes inside a character class (`/[/"']/`) are literal, so the
/// class state has to be tracked or the literal ends in the wrong place.
pub(crate) fn skip_regex_literal(bytes: &[u8], start: usize) -> usize {
    let mut index = start + 1;
    let mut inside_character_class = false;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = (index + 2).min(bytes.len()),
            b'[' => {
                inside_character_class = true;
                index += 1;
            }
            b']' if inside_character_class => {
                inside_character_class = false;
                index += 1;
            }
            // An unterminated literal was a division after all. Stop here so the
            // rest of the line is still scanned normally.
            b'\n' => return index,
            b'/' if !inside_character_class => {
                index += 1;
                break;
            }
            _ => index += 1,
        }
    }

    // Trailing flags (`/x/gi`) are part of the literal, not a new identifier.
    while bytes
        .get(index)
        .is_some_and(|byte| is_ident_continue_byte(*byte))
    {
        index += 1;
    }
    index
}

/// Walk a template literal starting at its opening backtick.
///
/// Returns the index just past the closing backtick together with the code
/// ranges of each `${ … }` interpolation, so callers can scan those as code
/// instead of treating the whole literal as opaque text.
fn template_literal(bytes: &[u8], start: usize) -> (usize, Vec<(usize, usize)>) {
    let mut index = start + 1;
    let mut interpolations = Vec::new();
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = (index + 2).min(bytes.len()),
            b'`' => return (index + 1, interpolations),
            b'$' if bytes.get(index + 1) == Some(&b'{') => {
                let code_start = index + 2;
                let code_end = interpolation_end(bytes, code_start);
                interpolations.push((code_start, code_end));
                index = (code_end + 1).min(bytes.len());
            }
            _ => index += 1,
        }
    }
    (bytes.len(), interpolations)
}

/// Index of the `}` closing an interpolation whose code begins at `start`.
///
/// Braces inside nested strings, templates, and comments do not count, or a
/// literal such as `` `${obj["}"]}` `` would end the interpolation early and
/// desynchronize the rest of the scan.
fn interpolation_end(bytes: &[u8], start: usize) -> usize {
    let mut index = start;
    let mut depth = 1usize;
    while index < bytes.len() {
        if is_comment_start(bytes, index) {
            index = skip_comment(bytes, index);
            continue;
        }
        match bytes[index] {
            b'`' => index = template_literal(bytes, index).0,
            b'\'' | b'"' => index = skip_string(bytes, index),
            b'{' => {
                depth += 1;
                index += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return index;
                }
                index += 1;
            }
            _ => index += 1,
        }
    }
    bytes.len()
}

fn skip_string(bytes: &[u8], start: usize) -> usize {
    let quote = bytes[start];
    let mut index = start + 1;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index += 2;
            continue;
        }
        if bytes[index] == quote {
            return index + 1;
        }
        index += 1;
    }
    bytes.len()
}

fn is_line_prefix_whitespace(bytes: &[u8], index: usize) -> bool {
    bytes[..index]
        .iter()
        .rev()
        .take_while(|byte| **byte != b'\n')
        .all(|byte| byte.is_ascii_whitespace())
}

fn looks_like_jsx_at(bytes: &[u8], index: usize) -> bool {
    matches!(
        bytes.get(index + 1),
        Some(b'>') | Some(b'/') | Some(b'A'..=b'Z') | Some(b'a'..=b'z')
    )
}

fn previous_non_whitespace(bytes: &[u8], index: usize) -> Option<u8> {
    bytes[..index]
        .iter()
        .rev()
        .find(|byte| !byte.is_ascii_whitespace())
        .copied()
}

fn skip_identifier(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && is_ident_continue_byte(bytes[index]) {
        index += 1;
    }
    index
}

fn is_ident_start_byte(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$')
}

fn is_ident_continue_byte(byte: u8) -> bool {
    is_ident_start_byte(byte) || byte.is_ascii_digit()
}

fn is_quote(byte: u8) -> bool {
    matches!(byte, b'"' | b'\'' | b'`')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_static_dynamic_and_re_export_imports() {
        let ast = parse_module(
            r#"
import React from "react"
import "./global.css"
export { helper } from "./helper"
const lazy = import("./lazy")
const data = require("./data")
"#,
        );

        assert!(
            ast.imports
                .iter()
                .any(|edge| { edge.specifier == "react" && edge.kind == ImportKind::Static })
        );
        assert!(ast.imports.iter().any(|edge| {
            edge.specifier == "./global.css" && edge.kind == ImportKind::SideEffect
        }));
        assert!(
            ast.imports
                .iter()
                .any(|edge| { edge.specifier == "./helper" && edge.kind == ImportKind::ReExport })
        );
        assert_eq!(ast.dynamic_import_specifiers(), vec!["./lazy"]);
        assert!(ast.import_specifiers().contains(&"./data".to_string()));
    }

    #[test]
    fn records_transform_features() {
        let ast = parse_module(
            r#"
@sealed
const enum Mode { A }
export default function Page(props: Props) { return <main /> }
"#,
        );

        assert!(ast.has_decorators);
        assert!(ast.has_enums);
        assert!(ast.has_typescript);
        assert!(ast.has_jsx);
        assert!(ast.exports.contains(&"Page".to_string()));
    }

    #[test]
    fn ignores_type_only_imports() {
        let ast = parse_module(
            r#"
import type { PageProps } from "ruvyxa/config";
import { createElement } from "react";
"#,
        );

        assert_eq!(ast.import_specifiers(), vec!["react"]);
    }

    #[test]
    fn recognizes_every_runtime_default_export_form() {
        for source in [
            "export default function Page() { return <main /> }",
            "export default class Page {}",
            "export default () => <main />",
            "const Page = () => <main />;\nexport default Page",
            "function Page() {}\nexport { Page as default }",
            "function Page() {}\nexport { Page as default, Page as Other }",
            "export { default } from \"./page\"",
            "export * as default from \"./page\"",
            "export {\n  // the page component\n  Page as default,\n}",
            "export { Page as Other, Page as default }",
        ] {
            assert!(
                has_default_export(source),
                "should detect a default export in: {source}"
            );
        }
    }

    #[test]
    fn rejects_sources_without_a_runtime_default_export() {
        for source in [
            "export const title = 'Missing'",
            "export function Page() {}",
            "// export default function Page() {}",
            "/* export default function Page() {} */",
            "const help = \"export default function Page() {}\"",
            "export const defaultTitle = 'Missing'",
            "export { Page }",
            // Re-exporting another module's default under a new name does not
            // give this module a default export.
            "export { default as Page } from \"./page\"",
            // Type-only exports leave no runtime binding.
            "export type { Page as default } from \"./page\"",
            "export * from \"./page\"",
        ] {
            assert!(
                !has_default_export(source),
                "should not detect a default export in: {source}"
            );
        }
    }

    /// A regex literal containing a quote used to start a string skip that ran
    /// to end-of-file, so every import after it disappeared from the dependency
    /// graph and the module was never bundled.
    #[test]
    fn regex_literals_do_not_hide_later_imports() {
        let ast = parse_module(
            r#"
const QUOTED = /["']/g
const CLASS_SLASH = /[/"]/
import { helper } from "./helper"
export { shared } from "./shared"
const lazy = import("./lazy")
"#,
        );

        assert_eq!(
            ast.import_specifiers(),
            vec!["./helper", "./shared", "./lazy"],
            "a regex literal must not swallow the rest of the module"
        );
    }

    /// The same swallowing made `check` reject a valid page with RUV1004.
    #[test]
    fn regex_literals_do_not_hide_a_later_default_export() {
        for source in [
            "const RE = /[\"']/;\nexport default function Page() { return <main /> }",
            "const RE = /don't/;\nexport default function Page() {}",
            "const RE = /[/\"]/g;\nfunction Page() {}\nexport { Page as default }",
        ] {
            assert!(has_default_export(source), "should detect: {source}");
        }
    }

    /// Interpolations are code. Treating a template literal as opaque text hid
    /// `${require("server-only")}` from the RUV1007 boundary check and dropped
    /// real dependency edges from the graph.
    #[test]
    fn template_interpolations_are_scanned_as_code() {
        let ast = parse_module(
            r#"
const loader = `${require("server-only")}`
const nested = `outer ${cond ? `inner ${import("./lazy")}` : ""} tail`
const text = `import "not-an-import" and require("not-either")`
"#,
        );

        let specifiers = ast.import_specifiers();
        assert!(
            specifiers.contains(&"server-only".to_string()),
            "{specifiers:?}"
        );
        assert!(specifiers.contains(&"./lazy".to_string()), "{specifiers:?}");
        assert!(
            !specifiers.contains(&"not-an-import".to_string()),
            "literal template text is not code: {specifiers:?}"
        );
        assert!(
            !specifiers.contains(&"not-either".to_string()),
            "literal template text is not code: {specifiers:?}"
        );
    }

    /// Every helper reached from an interpolation is bounded to that
    /// interpolation. Reading from the unbounded source let a keyword at the end
    /// of `${…}` pull its specifier out of the surrounding template text, which
    /// is literal text and not an import at all.
    #[test]
    fn interpolation_scans_do_not_read_past_their_own_range() {
        let ast = parse_module(r#"const trap = `${import}` from "./not-an-import"`"#);
        assert!(
            ast.import_specifiers().is_empty(),
            "text after an interpolation is not its specifier: {:?}",
            ast.import_specifiers()
        );

        let ast = parse_module(r#"const trap = `${require} ("./nope")`"#);
        assert!(ast.import_specifiers().is_empty(), "{:?}", ast.imports);

        let ast = parse_module(r#"const trap = `${export} * from "./nope"`"#);
        assert!(ast.import_specifiers().is_empty(), "{:?}", ast.imports);

        // The bound must not cost a real interpolated import its specifier.
        let ast = parse_module(r#"const real = `${import("./lazy")}`"#);
        assert_eq!(ast.import_specifiers(), vec!["./lazy"]);
    }

    /// A brace inside a nested string must not end the interpolation early, or
    /// the scan resumes at the wrong offset and loses everything after it.
    #[test]
    fn braces_inside_interpolated_strings_do_not_end_the_interpolation() {
        let ast = parse_module(
            r#"
const label = `${obj["}"]} tail`
import { helper } from "./helper"
"#,
        );

        assert_eq!(ast.import_specifiers(), vec!["./helper"]);
    }

    /// A `/` after a value is division. Treating it as a regex would skip real
    /// code instead — the opposite failure, and just as silent.
    #[test]
    fn division_is_not_mistaken_for_a_regex_literal() {
        let ast = parse_module(
            r#"
const ratio = total / count
const scaled = (a + b) / 2
const indexed = list[0] / 2
import { helper } from "./helper"
"#,
        );

        assert_eq!(ast.import_specifiers(), vec!["./helper"]);
    }

    /// After a keyword a `/` really is a regex, even though the preceding byte
    /// is an identifier byte.
    #[test]
    fn regex_after_a_keyword_is_still_a_regex() {
        let ast = parse_module(
            r#"
function pattern() { return /["']/ }
import { helper } from "./helper"
"#,
        );

        assert_eq!(ast.import_specifiers(), vec!["./helper"]);
    }

    #[test]
    fn default_export_detection_survives_unterminated_clauses() {
        // Malformed input must return an answer rather than scan out of bounds.
        for source in ["export {", "export { Page as", "export *", "export"] {
            assert!(!has_default_export(source), "{source}");
        }
    }
}
