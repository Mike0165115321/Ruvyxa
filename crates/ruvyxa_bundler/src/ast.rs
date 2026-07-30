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

    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if is_comment_start(bytes, index) {
            index = skip_comment(bytes, index);
            continue;
        }
        if is_quote(bytes[index]) {
            index = skip_string(bytes, index);
            continue;
        }

        if bytes[index] == b'@' && is_line_prefix_whitespace(bytes, index) {
            ast.has_decorators = true;
            index += 1;
            continue;
        }
        if bytes[index] == b'<' && looks_like_jsx_at(bytes, index) {
            ast.has_jsx = true;
        }

        if !is_ident_start_byte(bytes[index]) {
            index += 1;
            continue;
        }

        let start = index;
        index = skip_identifier(bytes, index);
        let word = &source[start..index];
        match word {
            "import" => {
                if let Some(edge) = import_edge(source, index) {
                    ast.imports.push(edge);
                }
            }
            "require" if previous_non_whitespace(bytes, start) != Some(b'.') => {
                if let Some(specifier) = call_specifier(source, index) {
                    ast.imports.push(ImportEdge {
                        specifier,
                        kind: ImportKind::Require,
                    });
                }
            }
            "export" => {
                if let Some(edge) = export_edge(source, index) {
                    ast.imports.push(edge);
                }
                if let Some(name) = export_name(source, index) {
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

    ast
}

fn import_edge(source: &str, after_keyword: usize) -> Option<ImportEdge> {
    let bytes = source.as_bytes();
    let index = skip_whitespace_and_comments(bytes, after_keyword);
    if index >= bytes.len() || bytes[index] == b'.' {
        return None;
    }
    if bytes[index] == b'(' {
        return call_specifier(source, index).map(|specifier| ImportEdge {
            specifier,
            kind: ImportKind::Dynamic,
        });
    }
    if is_quote(bytes[index]) {
        return quoted_value_at(source, index).map(|specifier| ImportEdge {
            specifier,
            kind: ImportKind::SideEffect,
        });
    }
    if word_at(source, index) == Some("type") {
        return None;
    }
    let declaration_start = index;
    find_from_specifier(source, declaration_start).map(|specifier| ImportEdge {
        specifier,
        kind: ImportKind::Static,
    })
}

fn export_edge(source: &str, after_keyword: usize) -> Option<ImportEdge> {
    let bytes = source.as_bytes();
    let index = skip_whitespace_and_comments(bytes, after_keyword);
    if word_at(source, index) == Some("type")
        || !matches!(bytes.get(index), Some(b'{') | Some(b'*'))
    {
        return None;
    }
    find_from_specifier(source, index).map(|specifier| ImportEdge {
        specifier,
        kind: ImportKind::ReExport,
    })
}

fn call_specifier(source: &str, after_keyword: usize) -> Option<String> {
    let bytes = source.as_bytes();
    let mut index = skip_whitespace_and_comments(bytes, after_keyword);
    if bytes.get(index) != Some(&b'(') {
        return None;
    }
    index = skip_whitespace_and_comments(bytes, index + 1);
    quoted_value_at(source, index)
}

fn find_from_specifier(source: &str, mut index: usize) -> Option<String> {
    let bytes = source.as_bytes();
    while index < bytes.len() {
        index = skip_whitespace_and_comments(bytes, index);
        if index >= bytes.len() || bytes[index] == b';' {
            return None;
        }
        if is_quote(bytes[index]) {
            index = skip_string(bytes, index);
            continue;
        }
        if word_at(source, index) == Some("from") {
            let value = skip_whitespace_and_comments(bytes, index + 4);
            return quoted_value_at(source, value);
        }
        index += 1;
    }
    None
}

fn quoted_value_at(source: &str, start: usize) -> Option<String> {
    let bytes = source.as_bytes();
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
    while index < bytes.len() {
        if is_comment_start(bytes, index) {
            index = skip_comment(bytes, index);
            continue;
        }
        if is_quote(bytes[index]) {
            index = skip_string(bytes, index);
            continue;
        }
        if !is_ident_start_byte(bytes[index]) {
            index += 1;
            continue;
        }
        let start = index;
        index = skip_identifier(bytes, index);
        if &source[start..index] == "export" && export_declares_default(source, index) {
            return true;
        }
    }
    false
}

/// Whether the export clause starting after `export` produces a default binding.
fn export_declares_default(source: &str, after_keyword: usize) -> bool {
    let bytes = source.as_bytes();
    let index = skip_whitespace_and_comments(bytes, after_keyword);

    // `export type { Page as default }` and `export type default` are erased at
    // compile time and leave no runtime binding behind.
    if word_at(source, index) == Some("type") {
        return false;
    }

    match word_at(source, index) {
        // `export default …`
        Some("default") => true,
        Some(_) => false,
        None => match bytes.get(index) {
            // `export * as default from "./page"`
            Some(b'*') => {
                let index = skip_whitespace_and_comments(bytes, index + 1);
                if word_at(source, index) != Some("as") {
                    return false;
                }
                let index = skip_whitespace_and_comments(bytes, index + "as".len());
                word_at(source, index) == Some("default")
            }
            // `export { Page as default }`, `export { default } from "./page"`
            Some(b'{') => named_clause_exports_default(source, index),
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
fn named_clause_exports_default(source: &str, brace: usize) -> bool {
    let bytes = source.as_bytes();
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

fn export_name(source: &str, after_keyword: usize) -> Option<String> {
    let bytes = source.as_bytes();
    let mut index = skip_whitespace_and_comments(bytes, after_keyword);
    for optional in ["default", "async"] {
        if word_at(source, index) == Some(optional) {
            index = skip_whitespace_and_comments(bytes, index + optional.len());
        }
    }
    let kind = word_at(source, index)?;
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

fn word_at(source: &str, start: usize) -> Option<&str> {
    let bytes = source.as_bytes();
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

    #[test]
    fn default_export_detection_survives_unterminated_clauses() {
        // Malformed input must return an answer rather than scan out of bounds.
        for source in ["export {", "export { Page as", "export *", "export"] {
            assert!(!has_default_export(source), "{source}");
        }
    }
}
