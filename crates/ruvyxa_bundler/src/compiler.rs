//! TypeScript and JSX compilation for the Ruvyxa Bundler.
//!
//! Ruvyxa owns module resolution, TypeScript build hooks, caching, boundary checks,
//! and linking. Oxc owns parsing, TypeScript stripping, JSX lowering, and
//! code generation for each source module.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use oxc::allocator::Allocator;
use oxc::codegen::Codegen;
use oxc::parser::Parser;
use oxc::semantic::SemanticBuilder;
use oxc::span::SourceType;
use oxc::transformer::{JsxRuntime as OxcJsxRuntime, TransformOptions, Transformer};
use rayon::prelude::*;

use crate::ast;
use crate::ast::ModuleAst;
use crate::cache::{CacheLookup, CompileCache};
use crate::hooks::{BuildHookContext, BuildHookPipeline};
use crate::resolver::ResolvedModule;
use crate::{BundleError, BundleInput, JsxRuntime, Result};

/// A compiled module: TypeScript/JSX has been converted to plain JavaScript.
///
/// Both heavy fields are shared rather than owned. A build holds the same module
/// in several collections at once — the full graph, the entry's static closure,
/// and one closure per emitted chunk — and each of those used to carry its own
/// copy of the generated JavaScript. Construct with [`CompiledModule::new`].
#[derive(Debug, Clone)]
pub struct CompiledModule {
    /// Canonical path (or virtual label for the synthetic entry).
    pub path: PathBuf,
    /// Plain JavaScript source after Oxc transformation.
    pub js: Arc<str>,
    /// Facts parsed from [`Self::js`], computed once at construction.
    pub ast: Arc<ast::ModuleAst>,
    /// Dependency paths preserved from the resolver stage.
    pub deps: Arc<[PathBuf]>,
    /// Exact source specifier to resolved path bindings from the resolver.
    pub dependency_aliases: Arc<BTreeMap<String, PathBuf>>,
    /// Whether this module comes from `node_modules` (external).
    pub is_external: bool,
    /// Whether this module's compiled output came from the compile cache.
    pub cache_hit: bool,
}

impl CompiledModule {
    /// Build a compiled module, scanning its output exactly once.
    ///
    /// The AST is parsed here — the single point where a `CompiledModule` comes
    /// into existence — so no later stage has a reason to walk `js` again. Chunk
    /// planning alone re-parsed every module once per dynamic root, once per
    /// emitted chunk, and twice more for the boundary check; that cost scaled
    /// with the number of `import()` sites in the app rather than with its size.
    pub(crate) fn new(
        path: PathBuf,
        js: String,
        deps: Vec<PathBuf>,
        dependency_aliases: BTreeMap<String, PathBuf>,
        is_external: bool,
        cache_hit: bool,
    ) -> Self {
        // External modules are excluded from linking, chunk closures, and the
        // boundary check, so their AST is never read. Scanning node_modules
        // output to fill a field nobody consults is pure cost.
        let ast = if is_external {
            Arc::new(ast::ModuleAst::default())
        } else {
            Arc::new(ast::parse_module(&js))
        };
        Self {
            path,
            js: js.into(),
            ast,
            deps: deps.into(),
            dependency_aliases: Arc::new(dependency_aliases),
            is_external,
            cache_hit,
        }
    }
}

struct CompiledModuleOutput {
    module: CompiledModule,
    hook_source_map: Option<String>,
}

pub(crate) fn compile_graph_with_hooks_and_maps(
    graph: &[ResolvedModule],
    input: &BundleInput,
    cache: &CompileCache,
    build_hooks: &BuildHookPipeline,
) -> Result<(Vec<CompiledModule>, BTreeMap<PathBuf, String>)> {
    reject_case_colliding_css_modules(graph, input)?;
    let results: Vec<Result<CompiledModuleOutput>> = graph
        .par_iter()
        .map(|module| compile_module(module, input, cache, build_hooks))
        .collect();

    let mut modules = Vec::with_capacity(results.len());
    let mut source_maps = BTreeMap::new();
    for output in results {
        let output = output?;
        if let Some(source_map) = output.hook_source_map {
            source_maps.insert(output.module.path.clone(), source_map);
        }
        modules.push(output.module);
    }
    Ok((modules, source_maps))
}

/// Scoped class names hash a case-folded project-relative path (so the same
/// file hashes identically across case-insensitive filesystems). Two
/// *distinct* CSS module files whose paths differ only by case would
/// therefore share every generated class name and silently swap styles —
/// reject that graph up front with an actionable error instead.
fn reject_case_colliding_css_modules(graph: &[ResolvedModule], input: &BundleInput) -> Result<()> {
    let mut seen: BTreeMap<String, &Path> = BTreeMap::new();
    for module in graph {
        if !crate::style_module::is_css_module_path(&module.path) {
            continue;
        }
        let key = crate::style_module::normalized_relative_path(&module.path, &input.project_root);
        match seen.get(&key) {
            Some(existing) if *existing != module.path.as_path() => {
                return Err(BundleError::Compiler(format!(
                    "CSS module paths {} and {} differ only by letter case and would generate identical scoped class names; rename one file",
                    existing.display(),
                    module.path.display()
                )));
            }
            Some(_) => {}
            None => {
                seen.insert(key, &module.path);
            }
        }
    }
    Ok(())
}

fn compile_module(
    module: &ResolvedModule,
    input: &BundleInput,
    cache: &CompileCache,
    build_hooks: &BuildHookPipeline,
) -> Result<CompiledModuleOutput> {
    let ext = module
        .path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    if crate::style_module::is_css_module_path(&module.path) {
        let css_module = crate::style_module::compile_css_module(&module.path, &input.project_root)
            .map_err(|error| {
                BundleError::Compiler(format!("{}: {error}", module.path.display()))
            })?;
        let js = crate::style_module::css_module_javascript(&css_module)
            .map_err(|error| BundleError::Compiler(error.to_string()))?;
        return Ok(CompiledModuleOutput {
            module: CompiledModule::new(
                module.path.clone(),
                js,
                module.deps.clone(),
                module.dependency_aliases.clone(),
                false,
                false,
            ),
            hook_source_map: None,
        });
    }

    let content_source = if let Some(compiled) = &module.compiled_content {
        compiled.to_string()
    } else if matches!(ext, "md" | "mdx") {
        crate::content::compile_content_module(&module.source, &module.path)
            .map_err(BundleError::Compiler)?
    } else {
        module.source.clone()
    };

    let hook_context = BuildHookContext {
        project_root: input.project_root.clone(),
        importer: Some(module.path.clone()),
        target: input.target,
    };
    let hook_output =
        build_hooks.transform_with_map(&content_source, &module.path, &hook_context)?;
    let source = hook_output.code;
    let hook_source_map = hook_output.map.or_else(|| module.load_source_map.clone());

    // Virtual entries and plain JavaScript pass through after registered transforms.
    if matches!(ext, "js" | "mjs" | "cjs") || module.path.to_string_lossy().contains("ruvyxa:") {
        return Ok(CompiledModuleOutput {
            module: CompiledModule::new(
                module.path.clone(),
                source,
                module.deps.clone(),
                module.dependency_aliases.clone(),
                module.is_external,
                false,
            ),
            hook_source_map,
        });
    }

    let transform_plan = ast::parse_module(&source);
    let has_jsx = matches!(ext, "tsx" | "jsx") || transform_plan.has_jsx;
    let jsx_runtime = input.options.jsx_runtime;

    match cache.lookup_with_options(&source, has_jsx, jsx_runtime) {
        CacheLookup::Hit(cached_js) => Ok(CompiledModuleOutput {
            module: CompiledModule::new(
                module.path.clone(),
                cached_js,
                module.deps.clone(),
                module.dependency_aliases.clone(),
                module.is_external,
                true,
            ),
            hook_source_map,
        }),
        CacheLookup::Miss(key) => {
            let js = transform_with_options(&source, has_jsx, jsx_runtime).map_err(|msg| {
                BundleError::Compiler(format!("{}: {}", module.path.display(), msg))
            })?;

            cache.store(&key, &js);

            Ok(CompiledModuleOutput {
                module: CompiledModule::new(
                    module.path.clone(),
                    js,
                    module.deps.clone(),
                    module.dependency_aliases.clone(),
                    module.is_external,
                    false,
                ),
                hook_source_map,
            })
        }
    }
}

/// Transform TypeScript/JSX source to plain JavaScript (classic JSX mode).
pub fn transform(source: &str, has_jsx: bool) -> std::result::Result<String, String> {
    transform_with_options(source, has_jsx, JsxRuntime::Classic)
}

/// Transform with explicit JSX runtime selection.
pub fn transform_with_options(
    source: &str,
    has_jsx: bool,
    jsx_runtime: JsxRuntime,
) -> std::result::Result<String, String> {
    // Preserve Ruvyxa's historical decorator contract: decorators are accepted
    // but removed without injecting an external runtime helper.
    let source = strip_decorators(source);
    let allocator = Allocator::default();
    let source_type = SourceType::mjs().with_typescript(true).with_jsx(has_jsx);
    let parsed = Parser::new(&allocator, &source, source_type).parse();
    if !parsed.diagnostics.is_empty() {
        return Err(format!(
            "Oxc could not parse TypeScript/JSX: {} syntax diagnostic(s)",
            parsed.diagnostics.len()
        ));
    }

    let mut program = parsed.program;
    let semantic = SemanticBuilder::new_compiler()
        .with_enum_eval(true)
        .build(&program);
    if !semantic.diagnostics.is_empty() {
        return Err(format!(
            "Oxc semantic analysis failed: {} diagnostic(s)",
            semantic.diagnostics.len()
        ));
    }

    let mut options = TransformOptions::default();
    options.jsx.runtime = match jsx_runtime {
        JsxRuntime::Classic => OxcJsxRuntime::Classic,
        JsxRuntime::Automatic => OxcJsxRuntime::Automatic,
    };
    options.jsx.jsx_plugin = has_jsx;
    options.jsx.throw_if_namespace = false;
    options.jsx.pure = false;
    options.typescript.optimize_const_enums = false;
    options.typescript.optimize_enums = false;

    let transformed = Transformer::new(&allocator, Path::new("ruvyxa:module.tsx"), &options)
        .build_with_scoping(semantic.semantic.into_scoping(), &mut program);
    if !transformed.diagnostics.is_empty() {
        return Err(format!(
            "Oxc TypeScript/JSX transform failed: {} diagnostic(s)",
            transformed.diagnostics.len()
        ));
    }

    Ok(Codegen::new().build(&program).code)
}

/// True when some line's first non-blank character is `@`.
///
/// A decorator is always the first thing on its line, so a file without such a
/// line has nothing to strip. Answering that with a plain line scan lets the
/// overwhelming majority of modules skip the tokenizer below entirely — they
/// are returned byte-for-byte, and no tokenizer limitation can reach them.
fn has_decorator_candidate(source: &str) -> bool {
    source
        .lines()
        .any(|line| line.trim_start().starts_with('@'))
}

/// Strip legacy decorators while preserving source line positions.
///
/// Oxc rejects legacy decorators, so they are removed before it parses. Finding
/// them means knowing whether an `@` is code, and getting that wrong is
/// expensive: a misplaced `@` deletion produces an unterminated string and a
/// parse failure that names the file rather than the construct that confused
/// the scan.
///
/// That question is not answered here. This used to carry its own tokenizer for
/// strings, template literals, and comments, which made it the crate's second
/// byte scanner — and it had exactly the half of the rules [`crate::ast`] was
/// missing, while missing the half `ast` had. It knew a `'` that does not close
/// before end-of-line is an apostrophe, not a delimiter; it did not know what a
/// regular expression is, so `` /`/ `` opened a template state that never
/// closed and the decorator on the next line survived into Oxc. Both halves now
/// live in the one scanner, and this asks it with [`ModuleAst::is_code_offset`]
/// instead of re-deriving the answer.
///
/// A removed decorator leaves behind exactly the newlines it spanned, so every
/// later line keeps its original number.
fn strip_decorators(source: &str) -> String {
    // A plain line scan settles the overwhelming majority of modules without
    // parsing anything: a decorator is always the first thing on its line.
    if !has_decorator_candidate(source) {
        return source.to_string();
    }

    let ast = ast::parse_module(source);
    if !ast.has_decorators {
        return source.to_string();
    }

    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut out = Vec::with_capacity(len);
    let mut i = 0;

    while i < len {
        if bytes[i] == b'@' && starts_line(bytes, i) && ast.is_code_offset(i) {
            let end = skip_decorator(bytes, i, &ast);
            // Emit the newlines the decorator spanned and nothing else. A
            // decorator on its own line leaves the line blank; a multi-line
            // `@Component({...})` leaves as many blank lines as it occupied.
            // Every later line then keeps its original number, which is what
            // source maps and Oxc's diagnostics are read against.
            let spanned_lines = bytes[i..end].iter().filter(|byte| **byte == b'\n').count();
            out.resize(out.len() + spanned_lines, b'\n');
            i = end;
            continue;
        }

        out.push(bytes[i]);
        i += 1;
    }

    String::from_utf8(out).expect("scanner copies whole UTF-8 sequences verbatim")
}

/// True when only blanks separate `at` from the start of its line.
fn starts_line(bytes: &[u8], at: usize) -> bool {
    bytes[..at]
        .iter()
        .rev()
        .take_while(|byte| **byte != b'\n')
        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r'))
}

/// Return the index just past a decorator that begins at `at`.
///
/// Only parentheses in code positions close the argument list. A `)` inside a
/// string, template, comment, or regex within the arguments is text, and `ast`
/// is the one place that already knows which is which.
fn skip_decorator(bytes: &[u8], at: usize, ast: &ModuleAst) -> usize {
    let len = bytes.len();
    let mut i = at + 1;
    // Identifier bytes. Anything above ASCII is a continuation byte of a
    // non-ASCII identifier character, which JavaScript allows in a name.
    while i < len
        && (bytes[i].is_ascii_alphanumeric()
            || matches!(bytes[i], b'_' | b'$' | b'.')
            || bytes[i] >= 0x80)
    {
        i += 1;
    }
    if i >= len || bytes[i] != b'(' {
        return i;
    }

    let mut depth = 1usize;
    i += 1;
    while i < len && depth > 0 {
        if ast.is_code_offset(i) {
            match bytes[i] {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
        }
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_interface() {
        let out = transform("interface Foo { bar: string; }\nconst x = 1;", false).unwrap();
        assert!(!out.contains("interface Foo"));
        assert!(out.contains("const x = 1;"));
    }

    #[test]
    fn strips_type_annotation() {
        let out = transform("const x: number = 5;", false).unwrap();
        assert!(!out.contains(": number"));
        assert!(out.contains("const x"));
    }

    #[test]
    fn strips_generic_type_params() {
        let out = transform("const arr = new Array<number>();", false).unwrap();
        assert!(!out.contains("<number>"));
        assert!(out.contains("new Array()"));
    }

    #[test]
    fn transforms_classic_jsx() {
        let out = transform("const el = <Input disabled />;", true).unwrap();
        assert!(out.contains("React.createElement(Input"));
        assert!(out.contains("disabled: true"));
    }

    #[test]
    fn transforms_automatic_jsx() {
        let out = transform_with_options(
            "const el = <div><span/><span/></div>;",
            true,
            JsxRuntime::Automatic,
        )
        .unwrap();
        assert!(out.contains("_jsxs"));
        assert!(out.contains("react/jsx-runtime"));
    }

    #[test]
    fn strips_decorators() {
        let out = strip_decorators("@Injectable()\nclass Service {}");
        assert!(!out.contains("@Injectable"));
        assert!(out.contains("class Service"));
    }

    #[test]
    fn indented_decorators_are_still_stripped() {
        let out = strip_decorators("class Service {\n  @observable()\n  value = 1;\n}");
        assert!(!out.contains("@observable"));
        assert!(out.contains("value = 1;"));
    }

    /// An apostrophe in prose is not a string delimiter. Reading it as one used
    /// to leave the scanner mis-synchronized for the rest of the file, so the
    /// `@` of the next scoped import was deleted as a decorator and the module
    /// no longer parsed.
    #[test]
    fn apostrophes_in_comments_do_not_shift_the_scan() {
        let source = concat!(
            "/**\n",
            " * React's head hoisting.\n",
            " */\n",
            "import { Seo } from '@ruvyxa/react';\n",
            "export default function Page() { return Seo; }\n",
        );
        let out = strip_decorators(source);
        assert!(
            out.contains("'@ruvyxa/react'"),
            "scoped specifier must survive: {out}"
        );
        assert_eq!(out, source, "a file with no decorator must be unchanged");
    }

    /// A regex literal is not a template, a comment, or a string. When this
    /// carried its own tokenizer it had no regex state, so a regex body holding
    /// a `` ` ``, a `/*`, or a `//` opened a state that never closed and the
    /// decorator on the next line reached Oxc, which rejects it.
    #[test]
    fn regex_literals_do_not_hide_a_later_decorator() {
        for source in [
            "const tick = /`/;\n@Injectable()\nclass S {}\n",
            "const open = /[/*]/;\n@Injectable()\nclass S {}\n",
            "const line = /[//]/;\n@Injectable()\nclass S {}\n",
        ] {
            let out = strip_decorators(source);
            assert!(!out.contains("@Injectable"), "decorator survived:\n{out}");
            assert!(out.contains("class S {}"), "body lost:\n{out}");
            assert_eq!(
                source.lines().count(),
                out.lines().count(),
                "line drift for:\n{source}"
            );
        }
    }

    #[test]
    fn line_comment_apostrophes_do_not_shift_the_scan() {
        let source = "// doesn't matter\nimport x from '@scope/pkg';\n";
        assert_eq!(strip_decorators(source), source);
    }

    /// `@` inside a string or after other code is never a decorator.
    #[test]
    fn at_signs_that_do_not_start_a_line_are_preserved() {
        let source = "const to = 'user@example.com';\nconst pkg = '@scope/name';\n";
        assert_eq!(strip_decorators(source), source);
    }

    #[test]
    fn transform_accepts_comment_apostrophes_before_scoped_imports() {
        let source = concat!(
            "/* it's fine */\n",
            "import { Seo } from '@ruvyxa/react';\n",
            "export const meta = { title: 'x' };\n",
            "export default function Page() { return Seo; }\n",
        );
        assert!(
            transform(source, false).is_ok(),
            "comment apostrophe must not break parsing"
        );
    }

    /// The overwhelming majority of modules have no decorator. They must come
    /// back byte-for-byte, whatever they contain.
    #[test]
    fn sources_without_a_decorator_are_returned_unchanged() {
        for source in [
            "const label = <p>don't</p>;\nconst re = /['\"]/g;\n",
            "const css = `\n@media (min-width: 40rem) { a { color: red } }\n`;\n",
            "const email = 'user@example.com';\nconst pkg = '@scope/name';\n",
            "// it's fine\nimport x from '@scope/pkg';\n",
            "const s = \"a ` backtick inside a string\";\nconst t = 'and @here too';\n",
        ] {
            assert_eq!(strip_decorators(source), source, "source: {source}");
        }
        assert!(!has_decorator_candidate("const pkg = '@scope/name';\n"));
        assert!(has_decorator_candidate(
            "class S {\n  @observable x = 1;\n}\n"
        ));
    }

    /// An unclosed quote is prose, not a string. It must not carry the scan
    /// past its own line, or the decorator after it goes unstripped.
    #[test]
    fn unclosed_quotes_do_not_hide_a_later_decorator() {
        for source in [
            "const label = <p>don't</p>;\n@Injectable()\nclass S {}\n",
            "const re = /['\"]/g;\n@Injectable()\nclass S {}\n",
            "// TODO: it's broken\n@Injectable()\nclass S {}\n",
        ] {
            let out = strip_decorators(source);
            assert!(!out.contains("@Injectable"), "decorator survived:\n{out}");
            assert!(out.contains("class S {}"), "body lost:\n{out}");
        }
    }

    /// A line-leading `@` inside a template literal is CSS or prose, not a
    /// decorator, including across an interpolation.
    #[test]
    fn line_leading_at_inside_a_template_is_left_alone() {
        let source = concat!(
            "@Injectable()\n",
            "class S {\n",
            "  css = `\n",
            "@media (min-width: ${size}rem) {\n",
            "  a { content: '}' }\n",
            "}\n",
            "`;\n",
            "}\n",
        );
        let out = strip_decorators(source);
        assert!(!out.contains("@Injectable"), "decorator kept:\n{out}");
        assert!(out.contains("@media"), "template text lost:\n{out}");
        assert!(out.contains("${size}"), "interpolation lost:\n{out}");
    }

    #[test]
    fn decorator_arguments_and_dollar_names_are_consumed_whole() {
        let out = strip_decorators("@Inject$ed({ a: ')', b: `)` })\nclass S {}\n");
        assert_eq!(out, "\nclass S {}\n");
    }

    /// Every later line has to keep its original number, or Oxc's diagnostics
    /// and the emitted source map point at the wrong line.
    #[test]
    fn stripping_preserves_the_line_count() {
        for source in [
            "@A()\n@B\nclass S {\n  @C() m() {}\n}\n",
            "@Component({\n  selector: 'x',\n})\nclass S {}\n",
            "class S {\n  @observable()\n  value = 1;\n}\n",
        ] {
            let out = strip_decorators(source);
            assert!(!out.contains('@'), "decorators remain:\n{out}");
            assert_eq!(
                source.lines().count(),
                out.lines().count(),
                "line drift for:\n{source}"
            );
        }
    }

    #[test]
    fn non_ascii_source_survives_the_byte_scan() {
        let source = "// ค่าเริ่มต้น — em dash\n@Injectable()\nclass บริการ {}\n";
        let out = strip_decorators(source);
        assert!(out.contains("ค่าเริ่มต้น"));
        assert!(out.contains("class บริการ {}"));
        assert!(!out.contains("@Injectable"));
    }

    #[test]
    fn transforms_enums() {
        let out = transform("enum Direction { Up, Down = 5, Left }", false).unwrap();
        assert!(out.contains("Direction"));
        assert!(out.contains("Development") || out.contains("Up"));
    }

    #[test]
    fn strips_satisfies_expression() {
        let out = transform("const config = { port: 3000 } satisfies Config;", false).unwrap();
        assert!(!out.contains("satisfies"));
        assert!(out.contains("const config"));
    }

    #[test]
    fn transforms_fragments_and_nested_expressions() {
        let src = r#"const el = <><p>Rendered from <code>{'[...slug]'}</code></p></>;"#;
        let out = transform(src, true).unwrap();
        assert!(out.contains("React.Fragment"));
        assert!(out.contains("[...slug]"));
    }

    #[test]
    fn strips_destructured_param_type_before_jsx() {
        let src = r#"export default function Page({ params }: { params: { slug: string } }) {
  return <main>{params.slug}</main>
}"#;
        let out = transform(src, true).unwrap();
        assert!(!out.contains(": { params"));
        assert!(out.contains("React.createElement(\"main\""));
    }
}
