# Markdown, MDX, Images & Metadata

Ruvyxa treats `page.md` and `page.mdx` as first-class route components -- no separate compile step,
no boilerplate. The bundler lowers markdown into React `createElement` calls at build time, feeds
the result through the existing TypeScript/JSX pipeline, and produces the same server/client splits
that `page.tsx` routes use.

---

## What You Will Learn

- Markdown routing: `page.md` and `page.mdx` as route files
- YAML frontmatter parsing and validation (RUV1312)
- MDX ESM blocks, JSX expressions, inline components
- Auto-exports: `frontmatter`, `meta`, `headings`, `contentFormat`
- ESM deduplication: skip auto-exports when MDX already exports them
- AST node lowering: every Markdown AST node -> `React.createElement`
- Content cache: 512-entry LRU with blake3 hashing
- `<Image>` component: full props interface
- Image optimization pipeline: WebP conversion, responsive variants
- `<Seo>` component and JSON-LD structured data
- `meta` export merge algorithm with layout meta
- Error codes: RUV1310, RUV1311, RUV1312
- Complete troubleshooting reference

---

## Markdown Pages: `page.md`

Create a file at `app/about/page.md`:

```markdown
---
title: About Us
description: What makes Ruvyxa tick
---

## Our Mission

We believe web frameworks should be **fast**, **simple**, and **fun**.
```

Visit `/about` -- Ruvyxa renders it wrapped in your root layout. No extra code.

### How It Works

The content pipeline lives in `crates/ruvyxa_bundler/src/content.rs`. The function
`compile_content_module()` orchestrates the process:

1. **split_frontmatter()** -- strips BOM, scans for `---` delimiters, separates YAML from body.
   Returns `(Option<String>, String)`.
2. **parse_frontmatter()** -- parses YAML via `serde_yaml_ng::from_str()`. Returns
   `serde_json::Value` (must be a mapping). Error: RUV1312.
3. **markdown::to_mdast()** -- parses GFM (`.md`) or MDX (`.mdx`) into an AST tree.
4. **collect_ast_headings()** -- walks AST, collects `{depth, text, slug}` for every heading.
5. **collect_definitions()** -- gathers `[label]: url` reference definitions into a BTreeMap.
6. **render_children()** or **render_node()** -- recursive AST walk, emitting strings of
   `React.createElement()` calls.
7. **module_source()** -- assembles the final ESM module: `import React`, auto-exports, default
   component.

```
                  +-----------------------------+
page.md (raw)     |  ---                        |
                  |  title: About Us            |
                  |  ---                        |
                  |                             |
                  |  ## Our Mission             |
                  |  We believe...              |
                  +------+----------------------+
                         |
                         v
   +-------------------------------------------------------+
   | compile_content_module()                               |
   |  1. split_frontmatter -> (Option<YAML>, body)          |
   |  2. parse_frontmatter -> serde_yaml_ng::Value          |
   |  3. markdown::to_mdast -> GFM AST                      |
   |  4. collect_ast_headings -> [{depth, text, slug}]      |
   |  5. render_children -> walk AST -> createElement calls  |
   |  6. module_source -> assemble ESM module                |
   +------+------------------------------------------------+
          |
          v
   +-------------------------------------------------------+
   | Generated ESM module                                   |
   |                                                        |
   | import React from "react";                             |
   | export const frontmatter = {"title":"..."};             |
   | export const meta = frontmatter;                       |
   | export const headings = [...];                         |
   | export const contentFormat = "md";                     |
   | export default function RuvyxaContentPage() {          |
   |   return React.createElement("article", {              |
   |     className: "ruvyxa-content",                       |
   |     "data-content-format": "md"                        |
   |   }, ...children...);                                  |
   | }                                                      |
   +------+------------------------------------------------+
          |
          v
   +-------------------------------------------------------+
   | Oxc compiler pipeline (same as page.tsx)                |
   | resolve -> compile -> boundary check -> link -> minify  |
   +-------------------------------------------------------+
```

The compiled output is wrapped in `<article className="ruvyxa-content">`. The `data-content-format`
attribute distinguishes `.md` from `.mdx`.

---

## MDX Pages: `page.mdx`

MDX = Markdown + JSX. Create `app/blog/hello.mdx`:

```mdx
---
title: Hello World
date: 2026-07-29
tags: [getting-started, mdx]
---

import { useState } from 'react'

export function Counter() {
  const [count, setCount] = useState(0)
  return <button onClick={() => setCount((c) => c + 1)}>Count: {count}</button>
}

## First Post

<Counter />
```

The Counter component is defined and used inside the same file.

### MDX Compilation Pipeline

For .mdx files the pipeline adds extra steps:

1. **split_frontmatter()** -- same as .md
2. **mdx_parse_options()** -- enables `mdx_esm`, `mdx_expression_flow/text`, `mdx_jsx_flow/text`.
   Disables `autolink`, `code_indented`, `html_flow`, `html_text`.
3. **markdown::to_mdast()** with MDX constructs -- produces MDX-specific AST nodes
4. **collect_mdx_esm()** -- extracts all `MdxjsEsm` nodes, joins into single ESM preamble string
5. **collect_ast_headings()** -- same as .md
6. **collect_definitions()** -- gathers link/image reference definitions
7. **render_children()** -- walks AST, passing `mdx: true` so intrinsic tags become
   `(components["div"] || "div")` lookups
8. **parse_mdx_esm()** -- validates ESM blocks via Oxc parser; returns `MdxSignal::Eof` on syntax
   errors -> RUV1311
9. **module_source()** -- prepends `import React` + raw ESM, appends auto-exports with dedup

### MDX Parse Options (Rust)

```rust
fn mdx_parse_options() -> ParseOptions {
    let mut constructs = Constructs::gfm();
    constructs.autolink = false;
    constructs.code_indented = false;
    constructs.html_flow = false;     // XSS safety
    constructs.html_text = false;     // XSS safety
    constructs.mdx_esm = true;
    constructs.mdx_expression_flow = true;
    constructs.mdx_expression_text = true;
    constructs.mdx_jsx_flow = true;
    constructs.mdx_jsx_text = true;
    ParseOptions {
        constructs,
        mdx_esm_parse: Some(Box::new(parse_mdx_esm)),
        ..ParseOptions::default()
    }
}
```

### MDX Features

| Feature             | Example                           | AST -> React                                  |
| ------------------- | --------------------------------- | --------------------------------------------- |
| JSX components      | `<Counter />`                     | `MdxJsxFlowElement` -> `render_mdx_element()` |
| Expressions         | `{2 + 2}` -> 4                    | `MdxFlowExpression` -> `(2 + 2)`              |
| ESM imports         | `import { Chart } from "./Chart"` | `collect_mdx_esm()` -> prepended as raw text  |
| Exports             | `export const name = "value"`     | Checked by `has_named_export()` for dedup     |
| Spread attrs        | `<Card {...props} />`             | `{...({props})}`                              |
| Fragment            | `<>...</>`                        | `name: None` -> `React.Fragment`              |
| Member expr         | `<Card.Header>`                   | `name.contains('.')` -> literal `Card.Header` |
| Component shadowing | `components.div` override         | `(components["div"]                           |     | "div")` |
| Boolean attrs       | `<Button disabled>`               | `{ disabled: true }`                          |
| JSX comment         | `{/* hidden */}`                  | `is_comment_only()` -> `"null"`               |

---

## Frontmatter

YAML delimited by `---` lines at the top of the file.

```yaml
---
title: My Page
description: A short summary
published: 2026-07-29
tags:
  - guide
  - mdx
author:
  name: Alice
  twitter: '@alice_dev'
draft: true
robots: noindex, nofollow
---
```

### Parsing Internals

```rust
fn split_frontmatter(source: &str) -> Result<(Option<String>, String), String> {
    // 1. Strip BOM (\\u{feff}) if present
    // 2. Check for leading "---\\n" or "---\\r\\n"
    // 3. Scan each line for closing "---" or "..."
    // 4. Preserve final line ending for YAML scalar chomping
    // 5. Return (None, source) if no frontmatter start
    // 6. Error RUV1312 if "---" opens but never closes
}

fn parse_frontmatter(source: Option<&str>) -> Result<Value, String> {
    // serde_yaml_ng::from_str()
    // Returns json!({}) for None/empty
    // Error RUV1312 if not a YAML mapping
}
```

Line splitting uses `split_inclusive('\\n')` -- `\\r\\n` preserved for Windows.

### Common Frontmatter Fields

| Field         | Type                     | Consumed by                      | Example                    |
| ------------- | ------------------------ | -------------------------------- | -------------------------- |
| `title`       | `string`                 | `<Seo>`, `meta`, layout, sitemap | `"About Us"`               |
| `description` | `string`                 | `<Seo>`, `meta`, search index    | `"A short summary"`        |
| `published`   | `date` string            | Feed, sitemap, JSON-LD           | `2026-07-29`               |
| `updated`     | `date` string            | Feed, sitemap, JSON-LD           | `2026-08-01`               |
| `tags`        | `string[]`               | Search index, content engine     | `["rust", "mdx"]`          |
| `author`      | `object{name, twitter?}` | Article JSON-LD, feed            | `{name: "Alice"}`          |
| `image`       | `string`                 | OG image, social card            | `/blog/hero.jpg`           |
| `draft`       | `boolean`                | Excluded from production build   | `true`                     |
| `robots`      | `string`                 | `meta[name="robots"]`            | `"noindex, nofollow"`      |
| `canonical`   | `string`                 | Canonical URL                    | `https://example.com/page` |

### Edge Cases

| Case               | Behavior                          |
| ------------------ | --------------------------------- |
| No frontmatter     | Returns `json!({})`               |
| Empty `---\\n---`  | Returns `json!({})`               |
| Comments only      | Stripped; result `json!({})`      |
| BOM prefix         | Stripped before check             |
| Closing with `...` | Supported (YAML doc-end)          |
| `\\r\\n` Windows   | Handled by split_inclusive        |
| Duplicate keys     | Last value wins                   |
| Non-mapping        | RUV1312: must be a YAML mapping   |
| Invalid YAML       | RUV1312: invalid YAML frontmatter |
| Unclosed           | RUV1312: no closing delimiter     |

---

## Auto-Exports

Every `page.md` and `page.mdx` auto-exports four named values. If the MDX ESM already exports the
same name, Ruvyxa skips the auto-export.

```tsx
import { frontmatter, meta, headings, contentFormat } from './page.mdx'
```

### TypeScript Type Definitions

```ts
type Frontmatter = Record<string, unknown>

type Meta = {
  title?: string
  description?: string
  image?: string
  [key: string]: unknown
}

type Heading = {
  depth: 1 | 2 | 3 | 4 | 5 | 6
  text: string
  slug: string
}

type Headings = Heading[]
type ContentFormat = 'md' | 'mdx'
```

### frontmatter

```ts
export const frontmatter = { title: 'Hello', published: '2026-07-29' }
```

`Record<string, unknown>`. Raw YAML parsed to JSON. All fields preserved.

### meta

```ts
export const meta = frontmatter
```

`Meta`. Defaults to frontmatter. Layouts override (see Metadata Inheritance).

### headings

```ts
export const headings = [
  { depth: 2, text: 'Intro', slug: 'intro' },
  { depth: 3, text: 'Details', slug: 'details' },
]
```

`Heading[]`. Field is `slug`, not `id`. Deduplicated by HeadingSlugger.

### contentFormat

```ts
export const contentFormat = 'md' // or "mdx"
```

`"md" | "mdx"`.

### ESM Deduplication

`has_named_export()` in content.rs scans JS tokens for these export forms:

| MDX ESM Pattern                               | Detected?      |
| --------------------------------------------- | -------------- |
| `export const headings = [...]`               | Yes            |
| `export function meta() {}`                   | Yes            |
| `export async function frontmatter() {}`      | Yes            |
| `export class contentFormat {}`               | Yes            |
| `export { custom as headings }`               | Yes            |
| `// export const meta` (commented)            | No             |
| `"export const frontmatter"` (string literal) | No             |
| `export type Meta = ...`                      | No (type-only) |

```mdx
---
title: Custom Meta
---

export const meta = {
  title: 'Custom Title',
  ogType: 'article',
}

# Page Content
```

Here `meta` uses the custom export. Other auto-exports still generated.

---

## AST Node Lowering

Every Markdown AST node is lowered to a `React.createElement()` call.

### Block-level Nodes

| Markdown      | AST Node                          | Generated React                                      |
| ------------- | --------------------------------- | ---------------------------------------------------- |
| root          | `Node::Root`                      | `React.createElement(React.Fragment, null, ...)`     |
| paragraph     | `Node::Paragraph`                 | `React.createElement("p", null, ...)`                |
| heading H1-H6 | `Node::Heading`                   | `React.createElement("h{depth}", { id: slug }, ...)` |
| blockquote    | `Node::Blockquote`                | `React.createElement("blockquote", null, ...)`       |
| ul            | `Node::List{ordered:false}`       | `React.createElement("ul", { className? }, ...)`     |
| ol            | `Node::List{ordered:true, start}` | `React.createElement("ol", { start }, ...)`          |
| li            | `Node::ListItem`                  | `React.createElement("li", null, ...)`               |
| task li       | `Node::ListItem{checked}`         | `li.task-list-item` + checkbox input                 |
| code block    | `Node::Code{lang, value}`         | `pre > code.language-{lang}`                         |
| table         | `Node::Table`                     | `table > thead+tbody > tr > th/td`                   |
| hr            | `Node::ThematicBreak`             | `React.createElement("hr", null)`                    |
| raw HTML      | `Node::Html`                      | `React.createElement("span", null, "escaped")`       |
| math display  | `Node::Math`                      | `div.math.math-display`                              |
| footnote def  | `Node::FootnoteDefinition`        | `aside#fn-{id}[role="doc-footnote"]` + back-link     |
| MDX JSX flow  | `Node::MdxJsxFlowElement`         | `render_mdx_element(name, attrs, children)`          |

### Inline-level Nodes

| Markdown       | AST Node                                  | Generated React                                                              |
| -------------- | ----------------------------------------- | ---------------------------------------------------------------------------- |
| text           | `Node::Text`                              | `"text"` (JS string)                                                         |
| emphasis       | `Node::Emphasis`                          | `React.createElement("em", null, ...)`                                       |
| strong         | `Node::Strong`                            | `React.createElement("strong", null, ...)`                                   |
| inline code    | `Node::InlineCode`                        | `React.createElement("code", null, "code")`                                  |
| link           | `Node::Link{url, title?}`                 | `React.createElement("a", { href, title? }, ...)`                            |
| image          | `Node::Image{url, alt, title?}`           | `React.createElement("img", { src, alt, loading:"lazy", decoding:"async" })` |
| br             | `Node::Break`                             | `React.createElement("br", null)`                                            |
| del            | `Node::Delete`                            | `React.createElement("del", null, ...)`                                      |
| inline math    | `Node::InlineMath`                        | `span.math.math-inline`                                                      |
| footnote ref   | `Node::FootnoteReference`                 | `sup#fnref-{id} > a[href="#fn-{id}"]`                                        |
| link ref       | `Node::LinkReference`                     | `a` with definition resolution                                               |
| image ref      | `Node::ImageReference`                    | `img` with definition or alt text                                            |
| strikethrough  | `Node::Strikethrough`                     | `React.createElement("del", null, ...)`                                      |
| MDX expr       | `MdxFlowExpression` / `MdxTextExpression` | `({expr})`                                                                   |
| MDX inline JSX | `Node::MdxJsxTextElement`                 | `render_mdx_element()`                                                       |

### Filtered Nodes

| Node               | Reason                             |
| ------------------ | ---------------------------------- |
| `Node::MdxjsEsm`   | Collected as ESM preamble          |
| `Node::Yaml`       | Already parsed                     |
| `Node::Toml`       | Not supported                      |
| `Node::Definition` | Collected for reference resolution |

### XSS Safety

Raw HTML is rendered as escaped text, never `dangerouslySetInnerHTML`:

```rust
Node::Html(value) => format!(
    "React.createElement(\\"span\\", null, {})",
    js_string(&value.value)  // serde_json escaping
)
```

### Heading Slug Generation

```rust
fn slugify(value: &str) -> String {
    // lowercase, alphanumeric keep, others -> hyphen, collapse, trim
    // fallback "section" for no alphanumeric chars
}
```

```
"Hello World"     -> "hello-world"
"What's New?"     -> "whats-new"
"  spaces  "      -> "spaces"
"emoji-only"     -> "section"
"emoji-only 2"   -> "section-1"
```

### Table Alignment

```rust
match alignment {
    AlignKind::Left   => "{ style: { textAlign: \\"left\\" } }",
    AlignKind::Right  => "{ style: { textAlign: \\"right\\" } }",
    AlignKind::Center => "{ style: { textAlign: \\"center\\" } }",
    AlignKind::None   => "null",
}
```

---

## Content Cache

512-entry LRU with blake3 key, stored in `OnceLock<Mutex<ContentModuleCache>>`.

```rust
const CONTENT_CACHE_LIMIT: usize = 512;

struct ContentModuleCache {
    entries: HashMap<String, Arc<str>>,
    insertion_order: VecDeque<String>,
}
```

### Cache Key

```rust
fn content_cache_key(extension: &str, source: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(extension.as_bytes());  // "md" or "mdx"
    hasher.update(b"\\0");
    hasher.update(source.as_bytes());
    hasher.finalize().to_hex().to_string()
}
```

### Eviction

```
cache >= 512 entries -> pop_front() oldest -> remove from HashMap
```

### Performance

| Operation         | Time                                 |
| ----------------- | ------------------------------------ |
| Cache lookup      | < 1 microsecond                      |
| MD compile (avg)  | 50-200 microseconds                  |
| MDX compile (avg) | 100-500 microseconds                 |
| Arc<str> sharing  | Zero-copy across compilation threads |

---

## Image Component

`<Image>` from `@ruvyxa/react` optimizes images at build time and generates responsive WebP sources.

```tsx
import { Image } from '@ruvyxa/react'

export default function Gallery() {
  return (
    <div>
      <Image
        src="/photos/sunset.jpg"
        alt="Sunset over mountains"
        width={1200}
        height={800}
        priority
      />
    </div>
  )
}
```

### Full Props Interface

```ts
interface ImageProps {
  src: string
  alt: string
  width?: number
  height?: number
  priority?: boolean
  loading?: 'lazy' | 'eager'
  quality?: number
  format?: 'webp' | 'avif' | 'original'
  sizes?: string
  className?: string
}
```

### Props Reference

| Prop        | Type                                 | Default                            | Description                                                 |
| ----------- | ------------------------------------ | ---------------------------------- | ----------------------------------------------------------- |
| `src`       | `string`                             | required                           | Path in `/public`, must start with `/`                      |
| `alt`       | `string`                             | required                           | Accessibility text                                          |
| `width`     | `number`                             | inferred from file                 | Intrinsic width in pixels                                   |
| `height`    | `number`                             | inferred from file                 | Intrinsic height in pixels                                  |
| `priority`  | `boolean`                            | `false`                            | Preload `<link>`, `loading="eager"`, `fetchpriority="high"` |
| `loading`   | `"lazy"` \| `"eager"`                | `"lazy"`                           | Native lazy loading                                         |
| `quality`   | `number`                             | config default (82)                | Override quality 1-100                                      |
| `format`    | `"webp"` \| `"avif"` \| `"original"` | `"webp"`                           | Preferred output format                                     |
| `sizes`     | `string`                             | `"(max-width: 768px) 100vw, 50vw"` | Sizes attribute for srcset                                  |
| `className` | `string`                             | --                                 | CSS class                                                   |

### Image Optimization Pipeline

The optimizer lives in `crates/ruvyxa_cli/src/image_optimizer.rs`.

```
public/photos/sunset.jpg (1.2 MB)
       |
       v
discover_sources() -- WalkDir::new(public)
  All files collected, sorted
       |
       v
ensure_unique_outputs() -- case-insensitive collision check
  hero.png + hero.PNG -> error: "image output collision"
       |
       v
process_one() -- per image
  1. read bytes
  2. image::load_from_memory() -- fallback copy unchanged on failure
  3. blake3 cache check
  4. encode_webp() -> webp::Encoder (lossy or lossless)
  5. write cache entry (atomic tmp+rename)
  6. materialize_cached() -> hardlink/copy to assets/
  7. emit_variants() -> Rayon parallel per width < intrinsic
       |
       v
assets/images/ output
  sunset.webp      (48 KB)  <- primary
  sunset-640w.webp (24 KB)  <- variant
  sunset-750w.webp (28 KB)
  sunset-828w.webp (32 KB)
  sunset.jpg       (84 KB)  <- original (if keepOriginal: true)
  .ruvyxa-images.json       <- manifest
```

### Pipeline Components

| Stage       | Library                                 | Description                     |
| ----------- | --------------------------------------- | ------------------------------- |
| Decode      | `image` crate (Rust)                    | Load PNG/JPEG -> DynamicImage   |
| Encode      | `webp::Encoder::encode_simple()`        | WebP compression, quality 1-100 |
| Resize      | `image::imageops::FilterType::Lanczos3` | High-quality downscale          |
| Cache       | blake3 content-addressed                | Avoid re-encoding               |
| Parallelism | Rayon `par_iter()`                      | Multi-threaded encoding         |

### Variant Widths

```rust
pub const DEFAULT_VARIANT_WIDTHS: [u32; 8] = [640, 750, 828, 1080, 1200, 1920, 2048, 3840];
```

Must match `packages/@ruvyxa/react/src/image.tsx`. Only widths strictly smaller than intrinsic width
are emitted.

### Config -- Rust Struct Mapping

```rust
pub struct ImageOptimizationOptions {
    pub optimize: bool,            // default: true
    pub quality: u8,               // default: 82, range: 1-100
    pub lossless: bool,            // default: false
    pub keep_original: bool,       // default: true
    pub variant_widths: Vec<u32>,  // default: 8 breakpoints
    pub parallelism: usize,        // default: 0 (Rayon global pool)
}
```

| TS Config Field       | Rust Field       | Type    | Default                                  | Description                       |
| --------------------- | ---------------- | ------- | ---------------------------------------- | --------------------------------- |
| `image.optimize`      | `optimize`       | `bool`  | `true`                                   | Enable WebP conversion            |
| `image.quality`       | `quality`        | `u8`    | `82`                                     | Quality 1-100                     |
| `image.lossless`      | `lossless`       | `bool`  | `false`                                  | Lossless only                     |
| `image.keepOriginal`  | `keep_original`  | `bool`  | `true`                                   | Keep original beside WebP         |
| `image.variantWidths` | `variant_widths` | `u32[]` | `[640,750,828,1080,1200,1920,2048,3840]` | Responsive breakpoints            |
| `image.workers`       | `parallelism`    | `usize` | `0`                                      | 0=Rayon global; >0=dedicated pool |

### keepOriginal Detail

When `true`, the original file is copied to assets beside the WebP. Without this, a CDN serving
`assets/` directly 404s on `<img src="/logo.png">` if not migrated to `<Image>`. The Rust dev server
has `resolve_public_asset` fallback -- CDNs do not.

Set `keepOriginal: false` when all references use `<Image>`.

### Image Error Conditions

| Error                      | Cause                                         | Fix                             |
| -------------------------- | --------------------------------------------- | ------------------------------- |
| image output collision     | `hero.png` and `hero.jpg` -> same `hero.webp` | Rename one                      |
| case collision (NTFS/APFS) | `Hero.png` and `hero.PNG`                     | Use unique names                |
| decode failure             | corrupted source                              | Replace file (copied unchanged) |
| non-optimizable type       | SVG, GIF, TIFF, BMP, ICO                      | Copied unchanged                |
| optimize: false            | pipeline disabled                             | Files copied unchanged          |

### Bypassed Image Detection

After optimization, `scan_raw_image_usage()` scans app files for raw `<img>` pointing at optimized
images:

```
app/page.tsx:42 <img src="/hero.png"> ships 84 KB instead of
generated WebP (48 KB). Use <Image> from @ruvyxa/react.
```

---

## SEO & Metadata

### The meta Export

Every page exports `meta`. Layouts merge child meta with their own.

```tsx
export default function BlogLayout({ children, meta }) {
  return (
    <>
      <header>
        <h1>{meta?.title ?? 'Blog'}</h1>
        {meta?.description && <p class="subtitle">{meta.description}</p>}
      </header>
      {children}
    </>
  )
}
```

### Metadata Inheritance Algorithm

```
Root Layout meta:  { title: "My Site" }
Blog Layout meta:  { title: "Blog", image: "/blog" }
MDX Page meta:     { title: "Hello World" }

Merge: deepest non-null non-undefined wins
Result: { title: "Hello World", image: "/blog" }
```

```ts
function mergeMeta(parent: Meta, child: Meta): Meta {
  const merged = { ...parent }
  for (const [key, value] of Object.entries(child)) {
    if (value !== null && value !== undefined) {
      merged[key] = value
    }
  }
  return merged
}
```

Rules:

- Child page `meta` overrides all parent layouts
- `null`/`undefined` values skipped (parent value preserved)
- Root layout owns final `<title>` and `<meta name="description">`
- Layouts receive child `meta` via props injection

### `<Seo>` Component

```tsx
import { Seo } from '@ruvyxa/react'

export default function BlogPost({ frontmatter }) {
  return (
    <>
      <Seo
        title={frontmatter.title}
        description={frontmatter.description}
        jsonLd={{
          '@context': 'https://schema.org',
          '@type': 'Article',
          headline: frontmatter.title,
          datePublished: frontmatter.published,
          author: { '@type': 'Person', name: frontmatter.author?.name },
        }}
        breadcrumbs={[
          { label: 'Home', href: '/' },
          { label: 'Blog', href: '/blog' },
          { label: frontmatter.title },
        ]}
      />
      <article>{/* content */}</article>
    </>
  )
}
```

### Seo Props Interface

```ts
interface SeoProps {
  title?: string
  description?: string
  image?: string
  jsonLd?: Record<string, unknown>
  breadcrumbs?: Array<{ label: string; href?: string }>
  canonical?: string
  noindex?: boolean
}
```

| Prop          | Type               | Description      | Schema Impact                              |
| ------------- | ------------------ | ---------------- | ------------------------------------------ |
| `title`       | `string`           | Page title       | `<title>`, `og:title`, `twitter:title`     |
| `description` | `string`           | Meta description | `meta[name=description]`, `og:description` |
| `image`       | `string`           | OG image URL     | `meta[property=og:image]`, `twitter:image` |
| `jsonLd`      | `object`           | JSON-LD          | `<script type="application/ld+json">`      |
| `breadcrumbs` | `{label, href?}[]` | BreadcrumbList   | Auto `BreadcrumbList` JSON-LD              |
| `canonical`   | `string`           | Canonical URL    | `<link rel="canonical">`                   |
| `noindex`     | `boolean`          | Robots noindex   | `<meta name="robots" content="noindex">`   |

### JSON-LD Schema Types

| Schema Type      | Required Fields                                  | Use Case          |
| ---------------- | ------------------------------------------------ | ----------------- |
| `Article`        | `headline`, `datePublished`, `author`            | Blog posts        |
| `BlogPosting`    | same + `dateModified`                            | Blog entries      |
| `Product`        | `name`, `image`, `description`, `offers`         | E-commerce        |
| `FAQPage`        | `mainEntity` (Question[])                        | FAQ pages         |
| `Recipe`         | `name`, `recipeIngredient`, `recipeInstructions` | Recipes           |
| `LocalBusiness`  | `name`, `address`, `telephone`                   | Business listings |
| `BreadcrumbList` | auto from `breadcrumbs` prop                     | Navigation        |
| `Organization`   | `name`, `url`, `logo`                            | Site-wide         |

### Metadata Inheritance Diagram

```
         +------------------+
         |  Root Layout      |  <Seo title="My Site" />
         |  meta: {          |
         |    title: "Site"  |
         |  }                |
         +------+-----------+
                |
         +------v-----------+
         |  Blog Layout      |  title = "Blog" (overrides root)
         |  meta: {          |
         |    title: "Blog"  |
         |    image: "/blog" |
         |  }                |
         +------+-----------+
                |
         +------v-----------+
         |  MDX Page         |  title = "Hello World" (overrides all)
         |  meta: {          |  image retained from blog
         |    title: "Hello" |
         |  }                |
         +------------------+
```

---

## Error Codes

### RUV1310

| Condition                 | Message                                             |
| ------------------------- | --------------------------------------------------- |
| Extension not .md or .mdx | `RUV1310: unsupported content extension for <path>` |
| .md GFM parse failure     | `RUV1310: Markdown parse error: <details>`          |

### RUV1311

| Condition          | Message                                                    |
| ------------------ | ---------------------------------------------------------- |
| .mdx parse failure | `RUV1311: MDX parse error: <details>`                      |
| Invalid ESM syntax | `MdxSignal::Eof("incomplete or invalid JS module syntax")` |

### RUV1312

| Condition            | Message                                                               |
| -------------------- | --------------------------------------------------------------------- |
| Unclosed frontmatter | `RUV1312: frontmatter starts with '---' but has no closing delimiter` |
| Invalid YAML         | `RUV1312: invalid YAML frontmatter: <details>`                        |
| Non-mapping          | `RUV1312: frontmatter must be a YAML mapping`                         |

---

## Troubleshooting

| Symptom                     | Cause                            | Fix                                   |
| --------------------------- | -------------------------------- | ------------------------------------- |
| Frontmatter not parsed      | Missing `---` closing            | Ensure closing `---` on own line      |
| MDX component not rendering | Missing import or `'use client'` | Client components need `'use client'` |
| Image not optimized         | Image outside `/public`          | Only `/public` images processed       |
| Slow build with many images | Default parallelism (CPU count)  | Set `image.workers: 2`                |
| JSON-LD not appearing       | `<Seo>` not rendered             | Must be in rendered tree              |
| `headings` array empty      | No Markdown `## ` syntax         | HTML `<h2>` in JSX not detected       |
| Image 404 in production     | `keepOriginal: false` on CDN     | Set `keepOriginal: true`              |
| `hero.webp` not generated   | Collision with `hero.jpg`        | Rename one                            |
| RUV1312 on valid YAML       | BOM or whitespace                | Ensure `---` on line 1                |
| Raw `<img>` warning         | Direct img tag                   | Use `<Image>` component               |

---

## Next Steps

- [02-routing.md](./02-routing.md) -- File-system routing
- [08-styling.md](./08-styling.md) -- Style content pages
- [11-configuration.md](./11-configuration.md) -- Full image config
- [14-plugins.md](./14-plugins.md) -- Content engine and sitemap
- [15-official-packages.md](./15-official-packages.md) -- Search index
