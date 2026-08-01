# Styling in Ruvyxa

Ruvyxa supports three CSS approaches out of the box: **Global CSS**, **SCSS/Sass**, and **CSS
Modules**. Use whatever fits your project — or mix them. All three support Hot Module Replacement
(HMR) in development and minification in production.

```
┌─────────────────────────────────────────────┐
│              Styling Options                 │
│                                              │
│  Global CSS     SCSS          CSS Modules    │
│  layout.tsx     .scss         .module.css    │
│  import './x.css'  auto-compile  .module.scss│
│                                              │
│  ────────── All get HMR ──────────           │
│  ──────── All work in dev + build ──────     │
└──────────────────────────────────────────────┘
```

---

## Type Definitions

### Config Types

```ts
// file: packages/@ruvyxa/core/src/types.ts
export interface RuvyxaConfig {
  css?: {
    /** Additional project-relative global stylesheet files or directories. */
    entries?: string[]
  }
}
```

### Style Collection Output

```rust
// file: crates/ruvyxa_dev_server/src/style.rs
pub struct StyleCollection {
    pub css: String,
    pub files: Vec<PathBuf>,
}
```

### CSS Module Output

```rust
// file: crates/ruvyxa_bundler/src/style_module.rs
pub struct CssModule {
    pub css: String,
    pub classes: BTreeMap<String, String>,
}
```

### Key Functions

```rust
// file: crates/ruvyxa_bundler/src/style_module.rs
pub fn is_css_module_path(path: &Path) -> bool
pub fn is_sass_path(path: &Path) -> bool
pub fn compile_sass_file(path: &Path, project_root: &Path) -> Result<String, String>
pub fn compile_css_module(path: &Path, project_root: &Path) -> Result<CssModule, String>
pub fn scope_css_module(css: &str, path: &Path, project_root: &Path) -> CssModule
pub fn css_module_javascript(module: &CssModule) -> Result<String, serde_json::Error>
pub fn minify_css(source: &str) -> String
```

### Internal Constants

```rust
// file: crates/ruvyxa_dev_server/src/style.rs
const SCRIPT_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx", "mts", "cts", "mjs", "cjs"];
const PREPROCESSOR_EXTENSIONS: &[&str] = &["scss", "sass", "less"];
```

---

## Global CSS

Import any `.css` file in a layout or page. It applies globally.

```css
/* app/global.css */
body {
  margin: 0;
  font-family: system-ui, sans-serif;
  background: #fafafa;
}

h1 {
  color: #1a1a1a;
}
```

```tsx
// app/layout.tsx — import global styles here
import './global.css'

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  )
}
```

The styles apply to every page in your app.

### Path Resolution

Global CSS imports are resolved relative to the importing file:

| Import                              | Resolved From                    |
| ----------------------------------- | -------------------------------- |
| `import "./global.css"`             | Same directory as importing file |
| `import "../styles/site.css"`       | Parent directory                 |
| `import "/styles/base.css"`         | Project root (`/`)               |
| `import "prismjs/themes/prism.css"` | `node_modules/`                  |
| `import "@styles/vars.css"`         | `tsconfig.json` `paths` alias    |

### Style Collection Algorithm

The style collection walks the import graph:

```
collect_styles(root, app_dir, entries):
  1. Walk app_dir for all script files (.ts, .tsx, .js, .jsx, ...)
  2. Parse each file for import statements
  3. For each import:
     a. If it's a .css/.scss/.sass file → add to style seeds
     b. If it's a preprocessor (.less) → error RUV1402
     c. If it's a script file within project → add to traversal queue
  4. Add explicit css.entries to style seeds
  5. For each style seed, recurse into @import directives
  6. Deduplicate by BTreeSet
  → Return concatenated CSS + file list
```

### Multiple Stylesheets

You can import multiple stylesheets — they are concatenated in order:

```tsx
import './reset.css'
import './typography.css'
import './layout.css'
```

The order within the bundle follows the import order.

---

## SCSS / Sass

Files ending in `.scss` are compiled automatically by the `grass` Sass crate. No extra config
needed.

```scss
// app/styles/variables.scss
$primary: #6366f1;
$secondary: #ec4899;
$spacing: 1rem;
```

```scss
// app/components/Card.scss
@use '../styles/variables' as v;

.card {
  border: 1px solid v.$primary;
  border-radius: 8px;
  padding: v.$spacing;

  &__title {
    color: v.$secondary;
    font-size: 1.25rem;
  }

  &__body {
    color: #333;
  }
}
```

Import SCSS files directly:

```tsx
import './Card.scss'

export function Card({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="card">
      <h2 className="card__title">{title}</h2>
      <div className="card__body">{children}</div>
    </div>
  )
}
```

### Grass Compiler Options

```rust
// file: crates/ruvyxa_bundler/src/style_module.rs
pub fn compile_sass_file(path: &Path, project_root: &Path) -> Result<String, String> {
    let options = grass::Options::default()
        .style(grass::OutputStyle::Expanded)   // Expanded output (not minified)
        .load_path(project_root)                // Project root for @use/@import resolution
        .load_path(project_root.join("node_modules"));  // node_modules for package imports
    grass::from_path(path, &options)
}
```

| Option      | Value           | Description                              |
| ----------- | --------------- | ---------------------------------------- |
| `style`     | `Expanded`      | Each rule on its own line, no compacting |
| `load_path` | Project root    | Resolves relative `@use`/`@import`       |
| `load_path` | `node_modules/` | Resolves npm Sass packages               |

### Sass Import Resolution

```rust
fn resolve_sass_import(root, base_dir, specifier) → Option<PathBuf>:
    if specifier starts with '.':
        base = base_dir / specifier    // relative import
    elif specifier starts with '/':
        base = root / specifier        // absolute from project root
    else:
        base = root / "node_modules" / specifier  // npm package

    // Candidates tried in order:
    candidates = [
        base,
        base + ".scss",
        base + ".sass",
        parent / "_" + name + ".scss",    // Sass partial
        parent / "_" + name + ".sass",
        base / "index.scss",
        base / "_index.scss",
        base / "index.sass",
        base / "_index.sass",
    ]
    → first candidate that exists as a file
```

### Supported Extensions

| Extension | Type          | Compilation                     |
| --------- | ------------- | ------------------------------- |
| `.scss`   | Sassy CSS     | Grass compiler                  |
| `.sass`   | Indented Sass | Grass compiler                  |
| `.css`    | Plain CSS     | None (passthrough)              |
| `.less`   | Less          | **Unsupported** — error RUV1402 |

### Unsupported Preprocessors

```rust
// Error RUV1402:
// "CSS preprocessor requires an explicit transform plugin"
// "Ruvyxa cannot safely treat `{specifier}` as plain CSS."
// Suggestion: "Compile Sass/Less to CSS first, or add a Ruvyxa transform plugin for that syntax."
```

---

## CSS Modules

Use `.module.css` or `.module.scss` for **scoped class names**. Each class gets a unique hash so
styles never leak.

```css
/* app/components/Button.module.css */
.primary {
  background: #6366f1;
  color: white;
  border: none;
  padding: 0.5rem 1rem;
  border-radius: 4px;
  cursor: pointer;
}

.primary:hover {
  background: #4f46e5;
}

.secondary {
  background: transparent;
  color: #6366f1;
  border: 1px solid #6366f1;
  padding: 0.5rem 1rem;
  border-radius: 4px;
}
```

```tsx
import styles from './Button.module.css'

export function Button({
  variant = 'primary',
  children,
}: {
  variant?: 'primary' | 'secondary'
  children: React.ReactNode
}) {
  return <button className={styles[variant]}>{children}</button>
}
```

Rendered HTML:

```html
<button class="Button_primary__a3f2k">Click me</button>
```

The class `Button_primary__a3f2k` is unique — no collisions with other `.primary` classes.

### CSS Module Path Detection

```rust
pub fn is_css_module_path(path: &Path) -> bool {
    let name = path.file_name().to_ascii_lowercase();
    name.ends_with(".module.css")
        || name.ends_with(".module.scss")
        || name.ends_with(".module.sass")
}
```

### Class Name Format

```
{stem}_{local}__{hex:016x}
  │       │        │
  │       │        └── fnv1a_64 hash (16 hex chars)
  │       └── local class name
  └── filename stem (without .module)
```

### Hashing Algorithm

```rust
fn scoped_class_name(path: &Path, project_root: &Path, local: &str) -> String {
    let relative = normalized_relative_path(path, project_root);
    // e.g. "app/components/button.module.css"
    let digest = fnv1a_64(format!("{relative}:{local}").as_bytes());
    // hash input: "app/components/button.module.css:primary"
    let stem = path.file_stem()
        .unwrap_or("style")
        .trim_end_matches(".module")
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    format!("{stem}_{local}__{digest:016x}")
}

fn fnv1a_64(input: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in input {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
```

The hash uses the **Fowler–Noll–Vo (FNV-1a) 64-bit** algorithm, seeded with the standard FNV-1a
offset basis `0xcbf29ce484222325` and prime `0x100000001b3`.

### Determinism

Class names are **deterministic across builds** — same file path + same class name = same hash every
time. This enables:

- Long-term caching of CSS files
- Reproducible builds
- Cross-platform consistency (paths are lowercased on all platforms)

```rust
fn normalized_relative_path(path, project_root) → String:
    canonicalize path and root
    strip root prefix
    convert to lowercase
    normalize separators to /
```

### Examples

| File                | Relative Path                  | Class      | Generated                          |
| ------------------- | ------------------------------ | ---------- | ---------------------------------- |
| `Button.module.css` | `components/button.module.css` | `.primary` | `Button_primary__feff5ad3a1e67b7b` |
| `Nav.module.scss`   | `components/nav.module.scss`   | `.link`    | `Nav_link__a1b2c3d4e5f6a7b8`       |
| `Card.module.css`   | `components/card.module.css`   | `.title`   | `Card_title__f1e2d3c4b5a69788`     |

### Cross-Runtime Golden Value

```rust
// Given root="/project", path="styles/card.module.css", class=".card"
// hash input: "styles/card.module.css:card"
// hash output: feff5ad3a1e67b7b
// Result: "card_card__feff5ad3a1e67b7b"
```

### `composes:` Support

CSS Modules support class composition:

```css
/* app/components/Typography.module.css */
.base {
  font-family: system-ui, sans-serif;
  line-height: 1.5;
}

.heading {
  composes: base;
  font-size: 2rem;
  font-weight: bold;
}

.subtitle {
  composes: base;
  font-size: 1.25rem;
  color: #666;
}
```

The composed class includes styles from both:

```tsx
<h1 className={styles.heading}>Title</h1>
<!-- Renders: class="Typography_base__a1b2c Typography_heading__d3e4f" -->
```

#### `composes:` Rules

| Rule             | Detail                                                             |
| ---------------- | ------------------------------------------------------------------ |
| **Scope**        | Only works within the **same** CSS Module file                     |
| **Cross-file**   | `composes: heading from "./other.module.css"` is **NOT supported** |
| **Format**       | `composes: className1 className2;` — space-separated               |
| **Order**        | Composed classes are appended after the base scoped name           |
| **Restrictions** | Must not use `from` keyword; all names must be valid CSS idents    |
| **Overrides**    | Properties in the composed class take precedence over base         |

#### Implementation

```rust
fn local_composition(chars, start) → Option<(end, Vec<String>)>:
    // Matches "composes:" keyword
    // Extracts space-separated class names before ";"
    // Returns list of composed class names

// composer scoped names are appended to the exporting class
for owner in owners:
    for scoped in composed:
        if not already present:
            exported.push(' ');
            exported.push(scoped);
```

### `:global()` Escape Hatch

Apply global (unscoped) styles from within a CSS Module:

```css
/* app/components/Layout.module.css */
.local {
  padding: 1rem;
}

:global(.markdown-content) h2 {
  border-bottom: 1px solid #ddd;
}

:global(.markdown-content) p {
  line-height: 1.7;
}
```

The `.markdown-content` styles are global — they apply to any element with that class, even from
other modules.

#### `:global()` Behavior

| Pattern                       | Scoped?                                                 | Example Output                     |
| ----------------------------- | ------------------------------------------------------- | ---------------------------------- |
| `.local`                      | Yes — hashed                                            | `.Layout_local__a1b2c`             |
| `:global(.global-class)`      | No — passed through                                     | `.global-class`                    |
| `:global(.theme-dark) .local` | Mixed — `:global()` selector unchanged, `.local` hashed | `.theme-dark .Layout_local__a1b2c` |

#### `:global()` Implementation

```rust
// Parser matches `:global(` and extracts contents up to matching `)`
// Contents are inserted verbatim into the output (no scoping)
```

---

## HMR for All Style Types

All three approaches support Hot Module Replacement:

1. **Edit** a `.css`, `.scss`, `.module.css`, or `.module.scss` file
2. **Save** — the development server attempts to update styles through its HMR path
3. **No page refresh** — no state loss, no flash

### HMR Conditions

| Change                                                 | Behavior                                             |
| ------------------------------------------------------ | ---------------------------------------------------- |
| Edit `.css` / `.scss` / `.module.css` / `.module.scss` | **Hot swap** — style updated in-place, no re-render  |
| Edit `.tsx` / `.ts` that imports a style               | Full component re-render with HMR                    |
| Add/remove style imports in source                     | **Full reload** — CSS module graph invalidated       |
| Add/remove CSS files                                   | **Full reload** — style collection cache invalidated |
| Edit non-CSS files                                     | No style impact                                      |

### Style Cache Invalidation

```rust
fn invalidate_styles_for_paths(paths) → bool:
    // Check if any changed path is in the cached style file set
    // If so, invalidate the CSS cache → next request recollects styles
    // Returns true if cache was invalidated (triggers HMR update)
```

---

## CSS Entries in Config

Sometimes you have stylesheets that are never imported by any component. For example, a third-party
CSS file or a global print stylesheet. Use `css.entries` in config:

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'

export default config({
  css: {
    entries: ['./node_modules/prismjs/themes/prism-tomorrow.css', './styles/print.css'],
  },
})
```

These stylesheets are included in every page, even though no component imports them directly.

### Entry Resolution

```rust
fn collect_explicit_entry(root, entry, styles) → Result:
    if entry is absolute:
        use as-is
    else:
        join(root, entry)

    if entry is outside project root:
        → error RUV1404

    if entry is a directory:
        walk directory recursively for .css/.scss/.sass files
        add all found files to styles

    if entry is a file:
        if extension is not .css/.scss/.sass:
            → error unsupported_preprocessor
        add file to styles

    if entry does not exist:
        → error RUV1403
```

### Source Comparison

| Source                  | How to include          | Use Case                                               |
| ----------------------- | ----------------------- | ------------------------------------------------------ |
| Imported in layout/page | `import "./styles.css"` | Component-specific or global styles used by components |
| External / unused       | `css.entries` in config | Third-party CSS, print stylesheets, analytics CSS      |

### Error Codes for CSS Entries

| Code    | Condition                | Message                                                  |
| ------- | ------------------------ | -------------------------------------------------------- |
| RUV1403 | Entry not found          | `Configured CSS entry was not found`                     |
| RUV1404 | Entry outside project    | `CSS entry must stay inside the project root`            |
| RUV1402 | Unsupported preprocessor | `CSS preprocessor requires an explicit transform plugin` |

---

## External CSS Imports

Import from `node_modules` the same way as local files:

```tsx
import 'prismjs/themes/prism-tomorrow.css'
```

Or add them as CSS entries if you need them globally without explicit imports:

```ts
css: {
  entries: ["prismjs/themes/prism-tomorrow.css"],
},
```

### Specifier Resolution

```rust
fn resolve_style_import(root, base_dir, specifier, tsconfig) → Option<PathBuf>:
    if specifier starts with '.':
        → base_dir / specifier          // relative
    elif specifier starts with '/':
        → root / specifier              // project-root absolute
    else:
        // Try tsconfig paths first
        if tsconfig.resolve(specifier) exists:
            return that path
        // Try project-relative path
        if (root / specifier) exists:
            return that
        // Fall back to node_modules
        return root / "node_modules" / specifier
```

---

## Style Objects (CSS-in-JS)

Ruvyxa provides an optional styling solution for those who prefer CSS-in-JS, but without the runtime
overhead.

```tsx
import { css } from '@ruvyxa/core/style'

const styles = {
  container: css({
    display: 'flex',
    padding: '20px',
    backgroundColor: 'var(--bg)',
    '&:hover': {
      backgroundColor: 'var(--bg-hover)',
    },
  }),
  title: css({
    fontSize: '24px',
    fontWeight: 'bold',
  }),
}

export default function Component() {
  return (
    <div className={styles.container}>
      <h1 className={styles.title}>Hello</h1>
    </div>
  )
}
```

These `css()` calls are statically extracted and compiled into regular CSS during build. They have
zero runtime cost.

---

## CSS Ordering

### Within Entry

Stylesheets are included in the order they're imported:

```tsx
import './reset.css' // 1st in bundle
import './typography.css' // 2nd
import './layout.css' // 3rd
```

### Within Bundle

1. Script-imported styles (in dependency-walk order)
2. `css.entries` styles (in array order)
3. Remote `@import` URLs are preserved in-place

### Specificity

CSS ordering in the bundle follows **source order**. If two rules have equal specificity, the one
that appears later wins. Since all styles are inlined into `<style>` tags, document order = source
import order.

---

## Tailwind CSS Support

Ruvyxa supports Tailwind CSS via the `@tailwindcss/cli` tool:

```css
/* app/global.css */
@import 'tailwindcss';
```

When `@import "tailwindcss"` is detected, the style collection compiles it through the Tailwind CLI:

```rust
fn compile_tailwind_css(root, input) → Result<String>:
    let tailwind = find_tailwind_cli(root)
    // Searches: node_modules/.bin/tailwindcss (or .cmd on Windows)
    // Runs: tailwindcss -i input --minify
    // Returns compiled CSS string
```

### Tailwind Setup Requirements

```
pnpm add tailwindcss
pnpm add -D @tailwindcss/cli
```

### Tailwind Error Codes

| Code    | Condition                    | Message                           |
| ------- | ---------------------------- | --------------------------------- |
| RUV1400 | Tailwind compilation failure | `Tailwind CSS compilation failed` |
| RUV1401 | CLI not found                | `Tailwind CSS CLI was not found`  |

---

## CSS Minification

In production mode, CSS is minified:

```rust
pub fn minify_css(source: &str) -> String {
    let no_comments = strip_css_comments(source);   // Remove /* ... */
    collapse_css_whitespace(&no_comments)            // Collapse whitespace
}
```

### Minification Conservatism

The minifier is intentionally conservative:

| Feature                 | Behavior                  |
| ----------------------- | ------------------------- |
| `/* comments */`        | Removed                   |
| Runs of whitespace      | Collapsed to single space |
| Spaces around `{}:;,()` | Removed                   |
| String literals         | Preserved verbatim        |
| `url()` values          | Preserved                 |
| Shorthand merging       | **Not performed**         |
| Selector optimization   | **Not performed**         |
| Color shortening        | **Not performed**         |

---

## Remote Style Imports

CSS `@import` URLs pointing to remote resources are preserved:

```css
/* Remote imports pass through unmodified */
@import "https://fonts.googleapis.com/css2?family=Inter";
@import "https://example.com/theme.css";
// @import "//cdn.example.com/shared.css";  // protocol-relative also preserved
```

Remote detection:

```rust
fn is_remote_style(specifier) -> bool:
    specifier starts with "http://"
        || "https://"
        || "//"
        || "data:"
```

---

## Style End Tag Escaping

To prevent XSS through CSS content, `</style` sequences in CSS are escaped:

```rust
fn escape_style_end_tags(css) -> String:
    // Replace every occurrence of "</style" (case-insensitive) with "<\/style"
    // This prevents premature closing of the <style> tag
```

---

## Error Codes Reference

| Code    | Condition                  | Message                                                  |
| ------- | -------------------------- | -------------------------------------------------------- |
| RUV1400 | Tailwind CLI error         | `Tailwind CSS compilation failed`                        |
| RUV1401 | Tailwind CLI not found     | `Tailwind CSS CLI was not found`                         |
| RUV1402 | Sass compilation error     | `Sass compilation failed`                                |
| RUV1402 | Unsupported preprocessor   | `CSS preprocessor requires an explicit transform plugin` |
| RUV1403 | Stylesheet not found       | `Stylesheet import could not be resolved`                |
| RUV1403 | CSS entry not found        | `Configured CSS entry was not found`                     |
| RUV1404 | Entry outside project root | `CSS entry must stay inside the project root`            |

---

## Performance Characteristics

| Operation           | Overhead                                 | Notes                            |
| ------------------- | ---------------------------------------- | -------------------------------- |
| Sass compilation    | Per file, cached                         | Rust-based compilation path      |
| CSS Module hashing  | Per class name                           | FNV-1a 64-bit deterministic hash |
| Style collection    | Walk import graph                        | Cached until invalidated         |
| CSS minification    | O(n) over CSS size                       | Conservative — no AST            |
| HMR style swap      | Depends on file, browser, and connection | WebSocket push + DOM update      |
| File system watcher | Per changed path                         | BTreeSet lookup                  |

### CSS Module Overhead

CSS Modules add a small per-class overhead:

- Each class name is hashed and mapped
- The class map is serialized to ESM: `export default {"btn": "Button_btn__abc123"}`
- The map is deduplicated within a file
- `composes:` adds whitespace-separated additional classes

### File Size Limits

There is no hard limit on CSS file size. However, large CSS files impact:

- Style collection time (proportional to file count + size)
- Bundle size (depends on the input, imports, and minifier configuration; measure the build output)
- Inline style tag size (inlined in HTML)

For very large stylesheets (>100 KiB), consider:

- Splitting into multiple files
- Using `@import` for logical separation
- Reviewing unused styles

---

## Edge Cases

| Scenario                                 | Behavior                                                           |
| ---------------------------------------- | ------------------------------------------------------------------ |
| **Same class in two modules**            | Different hashes — no collision                                    |
| **Empty CSS file**                       | Included but contributes nothing                                   |
| **Sass file with only variables**        | No output CSS; partials (`_file.scss`) tracked in dependency graph |
| **Circular Sass imports**                | Grass detects and errors                                           |
| **CSS `@import` of missing file**        | Error RUV1403                                                      |
| **File outside `app/` imported**         | Allowed if dependency-walked; must stay within project root        |
| **TS path alias to CSS**                 | Resolved via `tsconfig.json` `paths`                               |
| **Windows paths**                        | Normalized to forward slashes, lowercased                          |
| **`</style>` in CSS content**            | Escaped to `<\/style>`                                             |
| **Multiple `:global()` in one selector** | Each handled independently                                         |
| **`composes:` with non-existent class**  | No error — class simply not found in module's class map            |
| **Sass `@use` with `as *`**              | Namespace merged; works as expected                                |
| **CSS custom properties**                | Pass through unmodified                                            |
| **`@import` with media queries**         | Preserved: `@import url("print.css") print;`                       |
| **UTF-8 BOM in CSS**                     | Tolerated — Rust `fs::read_to_string` handles it                   |
| **Sass `@forward`**                      | Supported — grass forwards rules from partials                     |

---

## Full Example: Shop

Here is how you might combine different styling approaches in a real application:

```tsx
// app/layout.tsx
import './global.css' // Base reset, fonts, CSS variables

export default function RootLayout({ children }) {
  return (
    <html>
      <body className="bg-gray-100 text-gray-900">
        {' '}
        {/* Tailwind for utility */}
        {children}
      </body>
    </html>
  )
}
```

```tsx
// app/products/ProductCard.tsx
import styles from './ProductCard.module.scss' // SCSS Modules for complex component styles

export function ProductCard({ title, price }) {
  return (
    <div className={styles.card}>
      <h3 className={styles.title}>{title}</h3>
      <p className="font-bold text-lg">{price} THB</p> {/* Tailwind for one-offs */}
    </div>
  )
}
```

---

## Under the Hood: Reproducible Class Names

When using CSS Modules, class names are generated deterministically based on the file path and local
class name. This ensures they are identical across different machines and builds, preventing
hydration mismatches.

Algorithm:

1. Canonicalize the relative path (e.g. `app/components/Button.module.css`)
2. Hash the path using xxHash64
3. Combine: `[local-name]_[hash:5]`
4. Result: `btn_a7f3b`

This avoids the problem where different build environments generate different hashes, which can
cause caching issues and SSR hydration errors.

---

## Troubleshooting

| Problem                           | Likely cause                              | Fix                                                                |
| --------------------------------- | ----------------------------------------- | ------------------------------------------------------------------ |
| Styles not applying               | Wrong import path                         | Check the file extension and path                                  |
| CSS Module class is undefined     | Import name mismatch                      | `import styles from "./X.module.css"` — use `.module.` in filename |
| SCSS variables not working        | Missing `@use` instead of `@import`       | Use `@use "path" as *`                                             |
| Global styles override components | Specificity conflict                      | Move to CSS Module for scoping                                     |
| HMR not updating style            | CSS cache not invalidated                 | Save any `.tsx` file to trigger full HMR; or check file watcher    |
| Third-party CSS not loading       | Not imported or in entries                | Add to `css.entries` in config                                     |
| `composes:` not working           | Wrong module format                       | Only works in `.module.css` / `.module.scss`                       |
| Sass partial not found            | Wrong path resolution                     | Check partial naming (`_partial.scss`) and load paths              |
| Error RUV1402 on `.less` file     | Less is not supported                     | Compile to CSS first or use a plugin                               |
| Error RUV1403 on CSS import       | File doesn't exist                        | Check path and file extension                                      |
| Error RUV1404 on CSS entry        | Entry outside project root                | Move CSS file into project directory                               |
| Tailwind error RUV1400            | Tailwind config issue                     | Check `tailwind.config` and content paths                          |
| Tailwind error RUV1401            | `@tailwindcss/cli` not installed          | Run `pnpm add -D @tailwindcss/cli`                                 |
| Class names change between builds | Non-deterministic hashing                 | Check relative path stability (same project structure)             |
| Styles duplicated in bundle       | Same CSS file imported in multiple places | Deduplication handles this — BTreeSet prevents repeats             |

---

## Choosing an Approach

| When to use                          | Approach                                           |
| ------------------------------------ | -------------------------------------------------- |
| Quick prototyping, small projects    | Global CSS                                         |
| Nesting, variables, mixins           | SCSS                                               |
| Component libraries, large teams     | CSS Modules                                        |
| Scoped + nested                      | `.module.scss`                                     |
| Print styles, third-party CSS        | `css.entries` config                               |
| Tailwind utility classes             | Tailwind via `@import "tailwindcss"`               |
| Design system with tokens            | SCSS variables in partials                         |
| Micro-frontends or shared components | CSS Modules (scoped class names reduce collisions) |

---

## Best Practices

1. **Use CSS Modules by default.** Scoped styles prevent collisions and make refactoring safe.

2. **Keep global CSS minimal.** Only base resets and typography. Everything else should be scoped.

3. **Use SCSS variables for design tokens.** Colors, spacing, typography scale — define them once.

4. **Name modules after components.** `Button.module.css` lives alongside `Button.tsx`.

5. **Use `composes:` for shared styles.** Avoid duplicating style declarations.

6. **Use `:global()` sparingly.** It breaks scoping — only use it for truly global patterns (like
   markdown content).

7. **Use `css.entries` for external CSS.** Don't force a component import just to include a
   stylesheet.

8. **Colocate styles and components.** Put `Card.module.css` next to `Card.tsx` in the same folder.

9. **Use partials for shared Sass.** Prefix with `_` (`_variables.scss`, `_mixins.scss`) and `@use`
   where needed.

10. **Avoid deep selector nesting.** Sass nesting is convenient but generates high-specificity
    selectors. Keep 3 levels max.

11. **Test CSS in production build.** HMR hides some issues (ordering, missing files) that surface
    in production.

12. **Use `@use` not `@import` in Sass.** `@import` is deprecated in modern Sass and will be
    removed.

---

## Responsive Design Patterns

### Breakpoint Variables

```scss
// app/styles/breakpoints.scss
$mobile: 640px;
$tablet: 768px;
$desktop: 1024px;
$wide: 1280px;

@mixin mobile-only {
  @media (max-width: #{$tablet - 1px}) {
    @content;
  }
}

@mixin tablet-up {
  @media (min-width: $tablet) {
    @content;
  }
}

@mixin desktop-up {
  @media (min-width: $desktop) {
    @content;
  }
}

@mixin wide-up {
  @media (min-width: $wide) {
    @content;
  }
}
```

```scss
// app/components/Grid.module.scss
@use '../styles/breakpoints' as bp;

.grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1rem;

  @include bp.tablet-up {
    grid-template-columns: repeat(2, 1fr);
  }

  @include bp.desktop-up {
    grid-template-columns: repeat(3, 1fr);
  }
}
```

### Container Queries

```scss
.card {
  container-type: inline-size;

  @container (min-width: 400px) {
    .title {
      font-size: 1.5rem;
    }
  }
}
```

## Theming with CSS Custom Properties

Define theme tokens as custom properties:

```css
/* app/themes/light.css */
:root {
  --color-primary: #6366f1;
  --color-secondary: #ec4899;
  --color-bg: #ffffff;
  --color-text: #1a1a1a;
  --color-border: #e5e7eb;
  --spacing-unit: 0.25rem;
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.05);
  --shadow-md: 0 4px 6px rgba(0, 0, 0, 0.1);
}
```

```css
/* app/themes/dark.css */
[data-theme='dark'] {
  --color-primary: #818cf8;
  --color-secondary: #f472b6;
  --color-bg: #0f172a;
  --color-text: #e2e8f0;
  --color-border: #334155;
}
```

Use in components:

```scss
// app/components/ThemedCard.module.scss
.card {
  background: var(--color-bg);
  color: var(--color-text);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: calc(var(--spacing-unit) * 4);
  box-shadow: var(--shadow-md);
}
```

### Theme Toggle

```tsx
'use client'

import { useEffect, useState } from 'react'

export function ThemeToggle() {
  const [dark, setDark] = useState(false)

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', dark ? 'dark' : 'light')
  }, [dark])

  return <button onClick={() => setDark((d) => !d)}>{dark ? '☀️' : '🌙'}</button>
}
```

## CSS collection and inlining

Ruvyxa collects the stylesheets reached by the application imports and configured CSS entries, then
places the collected stylesheet in a `style[data-ruvyxa-css]` element in rendered documents. This is
stylesheet collection and inlining; it is not an above-the-fold critical-CSS extractor.

```text
imported CSS/SCSS + css.entries
          │
          ▼
collect_styles() → preprocess, resolve imports, scope modules
          │
          ▼
<style data-ruvyxa-css>…all collected CSS…</style>
```

The resulting HTML does not establish a universal performance score, eliminate every possible FOUC,
or guarantee that only visible-above-the-fold rules are included. If a project needs critical-CSS
extraction, run and measure a separate, application-specific tool after verifying that it preserves
the framework's style and hydration behavior.

## CSS Animation Performance

### GPU-Accelerated Properties

```css
/* ✅ GPU-accelerated — use these */
transform: translateX(100px);
transform: scale(1.5);
transform: rotate(45deg);
opacity: 0.5;

/* ❌ Layout-triggering — avoid in animations */
width: 50%;
height: 200px;
margin-left: 20px;
top: 100px;
```

### will-change Hint

```css
.animated-element {
  will-change: transform, opacity;
}
```

### Animation with CSS Modules

```scss
// app/components/FadeIn.module.scss
@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.wrapper {
  animation: fadeIn 0.3s ease-out;
}
```

## Print Stylesheets

```css
/* app/styles/print.css */
@media print {
  nav,
  footer,
  .sidebar,
  .no-print {
    display: none !important;
  }

  body {
    font-size: 12pt;
    color: #000;
    background: #fff;
  }

  a[href]::after {
    content: ' (' attr(href) ')';
  }
}
```

Add via config:

```ts
export default config({
  css: {
    entries: ['./styles/print.css'],
  },
})
```

## Font Loading

```css
/* app/global.css */
@import 'https://fonts.googleapis.com/css2?family=Inter:wght@400;500;700&display=swap';

body {
  font-family: 'Inter', system-ui, sans-serif;
}
```

Local fonts:

```css
@font-face {
  font-family: 'CustomFont';
  src: url('/fonts/custom.woff2') format('woff2');
  font-weight: 400;
  font-display: swap;
}
```

## Dark Mode with prefers-color-scheme

```scss
// app/styles/theme.scss
:root {
  --bg: #ffffff;
  --text: #1a1a1a;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg: #0f172a;
    --text: #e2e8f0;
  }
}
```

Combine with manual toggle:

```css
[data-theme='dark'] {
  --bg: #0f172a;
  --text: #e2e8f0;
}

/* prefers-color-scheme as default, manual toggle overrides */
@media (prefers-color-scheme: dark) {
  :root:not([data-theme='light']) {
    --bg: #0f172a;
    --text: #e2e8f0;
  }
}
```

## CSS Utility Classes

Create utility classes in global CSS:

```css
/* app/global.css */
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border-width: 0;
}

.truncate {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
```

## PostCSS and Autoprefixer

Ruvyxa applies autoprefixing automatically through the bundler during build. Vendor prefixes are
added based on the build target:

```ts
export default config({
  build: {
    target: 'es2020', // Controls CSS feature support level
  },
})
```

No separate PostCSS config is needed. Vendor prefixing is handled by the Rust bundler.

## CSS Ordering Deep Dive

### Sources in Order

The final CSS bundle order is:

1. **Remote `@import` URLs** — preserved at file level (not hoisted)
2. **App styles** — order matches script import traversal
3. **`css.entries`** — in array order, after all script-imported styles
4. **CSS Module scoped styles** — interleaved with their import order

### Priority Within Bundle

```tsx
// page.tsx
import './reset.css' // 1st in bundle
import './layout.css' // 2nd
import styles from './card.module.css' // 3rd (scoped)
```

Since all styles are in `<style>` tags in the `<head>`, document order determines cascade
resolution. For equal-specificity rules, later styles win.

## Build Output

During `ruvyxa build`, the style pipeline:

1. Collects all reachable stylesheets
2. Compiles Sass
3. Scopes CSS Modules
4. Resolves `@import` chains (except remote URLs)
5. Concatenates in order
6. Minifies (production only)
7. Inlines into each route's HTML

Individual CSS files are **not emitted** as separate bundle files. All CSS is inlined into the HTML
output. This eliminates render-blocking CSS requests at the cost of slightly larger HTML payloads.

---

## Try It Yourself

Build a styled card component using all three approaches.

**Step 1:** `app/global.css` — base styles:

```css
* {
  box-sizing: border-box;
}

body {
  margin: 0;
  font-family: system-ui, sans-serif;
  background: #f5f5f5;
  padding: 2rem;
}
```

**Step 2:** `app/components/Card.module.scss` — scoped styles with SCSS:

```scss
$radius: 12px;
$shadow: 0 2px 8px rgba(0, 0, 0, 0.1);

.card {
  background: white;
  border-radius: $radius;
  box-shadow: $shadow;
  overflow: hidden;
  max-width: 400px;
}

.header {
  padding: 1rem 1.5rem;
  background: #6366f1;
  color: white;
  font-weight: bold;
}

.body {
  padding: 1.5rem;
  line-height: 1.6;
}
```

**Step 3:** `app/components/Card.tsx`:

```tsx
import styles from './Card.module.scss'

export function Card({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className={styles.card}>
      <div className={styles.header}>{title}</div>
      <div className={styles.body}>{children}</div>
    </div>
  )
}
```

**Step 4:** Use it in a page:

```tsx
import { Card } from '../components/Card'

export default function Home() {
  return (
    <main>
      <Card title="Welcome to Ruvyxa">
        <p>This card is styled with a CSS Module using SCSS.</p>
        <p>The styles are scoped — no leaks, no conflicts.</p>
      </Card>
    </main>
  )
}
```

Open the browser — inspect the card. The class names are hashed and unique.

---

## How Style Collection Works in This Framework

Style collection starts from application scripts and follows their import graph. It also accepts
explicit project-relative `css.entries` from configuration, which is the right escape hatch for a
global stylesheet that is intentionally not imported by application code. A missing style import is
reported as `RUV1403`; adding a path to `css.entries` is appropriate only when the stylesheet is
truly global, not as a way to hide a broken relative import.

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'

export default config({
  css: {
    entries: ['styles/print.css', 'styles/tokens.css'],
  },
})
```

The paths are project-relative and validated with the rest of the configuration. Keep a component's
stylesheet imported by that component whenever possible; this preserves a readable dependency path
for both development and production collection.

### CSS Modules and Sass Have Separate Steps

Files ending in `.module.css`, `.module.scss`, or `.module.sass` are CSS Modules. Sass sources are
compiled first, then the framework scopes local class selectors using a stable project-relative path
and class name. The generated map is what the TypeScript module imports:

```tsx
import styles from './Button.module.scss'

export function Button({ children }: { children: React.ReactNode }) {
  return <button className={styles.primary}>{children}</button>
}
```

Use `:global(...)` only for a selector that must intentionally escape module scoping. It is not a
replacement for a global stylesheet; broad rules belong in an imported global CSS file or an
explicit `css.entries` file.

### What to Verify When a Style Does Not Appear

1. Check that the stylesheet is imported from a reachable application module, or listed in
   `css.entries`.
2. Check the exact relative path and extension; unresolved imports are not silently skipped.
3. For Sass, fix the compiler error rather than expecting a partial stylesheet.
4. Re-run the normal route/analysis checks after changing a shared style entry.

```bash
ruvyxa analyze --format human
ruvyxa trace /
npm run build
```

Development serves the collected CSS while watching files. Production rendering minifies the
collected CSS before embedding it in the document, so test a production build as well when order or
minification-sensitive rules are involved.

---

## Next Steps

- **[01-getting-started.md](./01-getting-started.md)** — Project setup
- **[02-routing.md](./02-routing.md)** — File-system routing
- **[03-server-client-components.md](./03-server-client-components.md)** — Server vs client
  components
