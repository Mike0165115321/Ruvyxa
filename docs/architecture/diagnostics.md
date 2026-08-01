# Diagnostics · การวินิจฉัย

**Crate**: `ruvyxa_diagnostics`  
**Module**: `crates/ruvyxa_diagnostics/src/lib.rs`

## สรุป

Central diagnostic types for the Ruvyxa framework. `Diagnostic` carries a structured error with
source span, import chain, suggested fix, and affected routes. `RuvyxaError` is the unified error
enum — wraps `Diagnostic`, `std::io::Error`, or a plain `String`. SARIF 2.1.0 serialization for CI
integration (GitHub Code Scanning, GitLab SAST).

---

## Core Data Structures

### SourceSpan

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub file: PathBuf,
    pub line: Option<u32>,
    pub column: Option<u32>,
}
```

Points to a source file, optionally with line/column. Both positional fields are `Option` — a bare
file reference (e.g. missing file) is valid.

### Diagnostic

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub title: String,
    pub explanation: String,
    pub span: Option<SourceSpan>,
    pub import_chain: Vec<PathBuf>,
    pub suggested_fix: Option<String>,
    pub affected_routes: Vec<String>,
}
```

| Field             | Type                 | Purpose                                    |
| ----------------- | -------------------- | ------------------------------------------ |
| `code`            | `&'static str`       | Error code, e.g. `"RUV1001"`               |
| `title`           | `String`             | Short human-readable summary               |
| `explanation`     | `String`             | Long-form why-this-happened                |
| `span`            | `Option<SourceSpan>` | Source location (file + optional line/col) |
| `import_chain`    | `Vec<PathBuf>`       | Import trace for boundary violations       |
| `suggested_fix`   | `Option<String>`     | How to resolve the issue                   |
| `affected_routes` | `Vec<String>`        | Routes impacted by this error              |

---

## Builder Pattern

```rust
Diagnostic::new(code, title)
    .explain("why")                        // set explanation
    .at_file("path/to/file.rs")            // set span, no line/col
    .at_file_with_span("path.rs", 42, 5)   // set span with line + col
    .suggest("move the import")            // set suggested_fix
```

Each builder method consumes and returns `Self` (not `&mut self`), enabling chaining. `at_file` and
`at_file_with_span` overwrite the span. All methods are additive — no validation or side effects.

---

## Display Format

```
CODE: title
File: /path/to/file.rs:42:5

Why:
  explanation text

Fix:
  suggested fix text

Affected routes:
  /blog/[slug]
  /about
```

Span line omission adjusts format: no line → `File: path`, line without column → `File: path:line`.
Sections omitted when empty.

---

## RuvyxaError

```rust
#[derive(Debug, Error)]
pub enum RuvyxaError {
    #[error("{0}")]
    Diagnostic(Box<Diagnostic>),

    #[error("{message}")]
    Io {
        message: String,
        #[source]
        source: std::io::Error,
    },

    #[error("{0}")]
    Message(String),
}
```

Three variants:

- **Diagnostic** — wraps a `Box<Diagnostic>`, delegates `Display` to Diagnostic's formatter.
- **Io** — structured I/O error preserving the source `std::io::Error`.
- **Message** — plain string fallback.

### Trait Impls

```rust
impl From<Diagnostic> for RuvyxaError   // wraps into Diagnostic variant
impl From<std::io::Error> for RuvyxaError // wraps into Io variant
pub type Result<T> = std::result::Result<T, RuvyxaError>;
```

---

## SARIF Integration

```rust
pub fn diagnostics_to_sarif(
    diagnostics: &[Diagnostic],
    tool_name: &str,
    tool_version: &str,
    project_root: &Path,
) -> serde_json::Value
```

Produces SARIF 2.1.0:

```json
{
  "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
  "version": "2.1.0",
  "runs": [
    {
      "tool": {
        "driver": {
          "name": "ruvyxa",
          "version": "0.1.0",
          "informationUri": "https://github.com/ruvyxa/ruvyxa",
          "rules": [
            {
              "id": "RUV1001",
              "name": "RUV1001",
              "shortDescription": { "text": "Private import" },
              "fullDescription": { "text": "A client module imports server-only code." },
              "defaultConfiguration": { "level": "error" },
              "help": { "text": "Move the import behind a server boundary." }
            }
          ]
        }
      },
      "results": [
        {
          "ruleId": "RUV1001",
          "level": "error",
          "message": { "text": "Private import: A client module imports server-only code." },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": { "uri": "app/page.tsx" },
                "region": { "startLine": 4, "startColumn": 7 }
              }
            }
          ],
          "properties": {
            "suggestedFix": "Move the import behind a server boundary.",
            "affectedRoutes": [],
            "importChain": []
          }
        }
      ]
    }
  ]
}
```

Key behavior:

- **Rules deduplicated** by `code` via `BTreeMap` — preserves insertion order, keeps first
  occurrence.
- **URIs project-relative** — each file path is stripped of `project_root` prefix after
  normalization.
- **`help` omitted** when `suggested_fix` is `None`.
- **`region` omitted** when span lacks line/column.
- **`properties`** carries `suggestedFix`, `affectedRoutes`, `importChain` as supplemental data.

---

## Error Code Catalog

| Code    | Title                                                           | Crate              |
| ------- | --------------------------------------------------------------- | ------------------ |
| RUV1001 | App directory was not found                                     | graph              |
| RUV1002 | Invalid dynamic route segment / Catch-all must be final segment | graph              |
| RUV1003 | Conflicting route paths                                         | graph              |
| RUV1004 | Page is missing a default export                                | graph, dev_server  |
| RUV1007 | Server-only module imported into client graph                   | graph              |
| RUV1008 | Private environment variable used in client graph               | graph              |
| RUV1009 | Client-only module imported into server/SSR graph               | graph, bundler     |
| RUV1010 | Server directory module reached by client graph                 | graph              |
| RUV1100 | React SSR failed                                                | dev_server         |
| RUV1102 | SSR renderer was not found                                      | dev_server         |
| RUV1200 | API route execution failed                                      | dev_server         |
| RUV1201 | No available server port was found                              | dev_server         |
| RUV1202 | API renderer was not found                                      | dev_server         |
| RUV1300 | Client hydration bundling failed / Compile error                | dev_server         |
| RUV1303 | Client route was not found                                      | dev_server         |
| RUV1304 | Client bundle requested for a non-page route                    | dev_server         |
| RUV1400 | Tailwind CSS compilation failed                                 | dev_server         |
| RUV1401 | Tailwind CSS CLI was not found                                  | dev_server         |
| RUV1402 | Sass compilation failed                                         | dev_server         |
| RUV1403 | CSS import / stylesheet could not be resolved                   | dev_server         |
| RUV1404 | CSS entry must stay inside the project root                     | dev_server         |
| RUV1500 | SSG render failed                                               | dev_server         |
| RUV1501 | Route action file was not found                                 | dev_server         |
| RUV1550 | PPR render failed                                               | dev_server         |
| RUV1600 | Config load failure                                             | cli/config         |
| RUV1601 | Config value or path is invalid                                 | cli/config         |
| RUV1602 | Config shape, unknown field, or configured limit is invalid     | cli/config         |
| RUV1603 | Adapter definition or output is invalid                         | cli/config/runtime |
| RUV1702 | Worker pool script was not found                                | dev_server         |
| RUV1101 | SSR renderer arguments are missing                              | runtime/SSR        |

This is a source-confirmed catalog of the codes documented by this architecture page, not a promise
that every runtime or package code is listed. Codes are string constants (`&'static str`), not enum
variants — any crate can emit a code without touching the diagnostics crate.

---

## Under the Hood

### `normalized_canonical_path`

```rust
pub fn normalized_canonical_path(path: &Path) -> PathBuf
```

Wraps `std::fs::canonicalize` then strips the Windows `\\?\` verbatim prefix on `cfg(windows)`.
Falls back to the original path when the file does not exist. Used inside SARIF serialization to
produce paths that JavaScript runtimes (Bun, Node) can pass to `pathToFileURL`.

### SARIF rule deduplication

Uses `BTreeMap<&str, &Diagnostic>` keyed by `code`. Because `BTreeMap` iterates in key order, rules
are sorted alphabetically by code in the output. The first diagnostic for each code is used as the
rule template — subsequent diagnostics with the same code are still emitted as separate results but
reference the same rule.

### Error scope

This crate owns only the type definitions and SARIF serializer. The actual error emission happens in
domain crates (`ruvyxa_graph`, `ruvyxa_bundler`, `ruvyxa_dev_server`) which construct `Diagnostic`
values directly via the builder pattern. There is no centralized error registry — codes are
conventional strings.
