# สร้าง Content Pages ด้วย Markdown, MDX และ Image

Ruvyxa รองรับการสร้างหน้าเนื้อหา (content pages) ด้วยไฟล์ Markdown (`.md`) และ MDX (`.page.mdx`)
ควบคู่กับระบบปรับรูปภาพอัตโนมัติครบวงจร — ตั้งแต่การ parse frontmatter, compilation MDX pipeline,
AST node lowering, 512-entry LRU cache, image optimization (oxipng/mozjpeg/guetzli/cwebp/libaom),
ไปจนถึง SEO structured data (JSON-LD)

---

## ภาพรวมระบบ MDX

Ruvyxa ใช้ MDX compiler แบบ full pipeline เพื่อแปลง `.page.mdx` และ `.md` เป็น React components

```
Source (.md / .page.mdx)
    │
    ▼
┌──────────────────────────────────────┐
│ 1. Frontmatter Parser                │
│    - serde_yaml / js-yaml            │
│    - schema validation               │
│    - export frontmatter object       │
└────────────┬─────────────────────────┘
             ▼
┌──────────────────────────────────────┐
│ 2. MDX Compiler (remark/rehype)      │
│    - MDAST ← MDX                     │
│    - JSX → createElement()           │
│    - ESM import resolution           │
│    - module deduplication            │
└────────────┬─────────────────────────┘
             ▼
┌──────────────────────────────────────┐
│ 3. AST Node Lowering                 │
│    - tables → <table> wrapper        │
│    - code blocks → <pre><code> +     │
│      syntax highlighting             │
│    - images → <Image> component      │
│    - checkboxes → task lists         │
│    - math → LaTeX rendering          │
│    - footnotes → section             │
└────────────┬─────────────────────────┘
             ▼
┌──────────────────────────────────────┐
│ 4. Auto-Export Injection             │
│    - frontmatter, meta, headings[]   │
│    - contentFormat                   │
│    - 512-entry LRU cache             │
└────────────┬─────────────────────────┘
             ▼
┌──────────────────────────────────────┐
│ 5. Image Optimization (build-time)   │
│    - decode → resize → encode        │
│    - mozjpeg (JPEG), oxipng (PNG)    │
│    - cwebp (WebP), libaom (AVIF)     │
│    - guetzli (premium JPEG)          │
│    - responsive srcset variants      │
│    - blake3-256 content hash         │
└────────────┬─────────────────────────┘
             ▼
┌──────────────────────────────────────┐
│ 6. SEO Engine                        │
│    - <Meta> component tags           │
│    - <Seo> component                 │
│    - JSON-LD (Article, Breadcrumb,   │
│      Organization, etc.)             │
└──────────────────────────────────────┘
```

### TypeScript Type Definitions เต็มรูปแบบ

```ts
// @ruvyxa/mdx — type declarations
declare module '*.page.mdx' {
  import { ComponentType } from 'react'

  // default export — component หลัก
  const MDXComponent: ComponentType<{}>
  export default MDXComponent

  // frontmatter — metadata จาก YAML header
  export const frontmatter: MDXFrontmatter

  // meta — route metadata
  export const meta: RouteMeta

  // headings — สารบัญอัตโนมัติ
  export const headings: Heading[]

  // contentFormat — ระบุชนิดไฟล์
  export const contentFormat: 'mdx' | 'markdown'
}

declare module '*.md' {
  export const frontmatter: Record<string, unknown>
  export const content: string
  export const contentFormat: 'markdown'
}

// ---- Type Definitions ----

interface MDXFrontmatter {
  title?: string
  description?: string
  publishedAt?: string // ISO 8601
  updatedAt?: string // ISO 8601
  author?: {
    name: string
    avatar?: string
    bio?: string
    twitter?: string
  }
  tags?: string[]
  image?: string // OG image path
  draft?: boolean // true → ข้ามตอน build
  noindex?: boolean // true → <meta name="robots" content="noindex">
  canonical?: string // canonical URL
  layout?: string // custom layout override
  [key: string]: unknown // custom fields
}

interface RouteMeta {
  path: string // /blog/hello-world
  slug: string // hello-world
  params: Record<string, string> // { slug: 'hello-world' }
  title?: string
  description?: string
}

interface Heading {
  depth: 1 | 2 | 3 | 4 | 5 | 6
  text: string
  id: string // slugified anchor id
}

interface OGImage {
  url: string
  width?: number
  height?: number
  alt?: string
}

interface JSONLDEntity {
  '@context': 'https://schema.org'
  '@type': string
  [key: string]: unknown
}
```

### `.page.mdx` vs `.md` — ตารางเปรียบเทียบ

| คุณสมบัติ                       | `page.mdx`                                 | `.md` (plain)        |
| ------------------------------- | ------------------------------------------ | -------------------- |
| JSX components                  | ✅                                         | ❌                   |
| ESM imports (`import ... from`) | ✅                                         | ❌                   |
| Expressions (`{1+2}`, `.map()`) | ✅                                         | ❌                   |
| Frontmatter YAML                | ✅                                         | ✅                   |
| Auto-exports                    | frontmatter, meta, headings, contentFormat | frontmatter, content |
| Rendering strategies            | SSR, SSG, ISR, PPR, CSR                    | SSR, SSG, ISR        |
| Image optimization              | ✅ (อัตโนมัติ)                             | ✅ (อัตโนมัติ)       |
| SEO metadata                    | ✅ (Meta/Seo component)                    | ✅ (จาก frontmatter) |

---

## Frontmatter YAML — เจาะลึก

### กลไกการ Parse

```
1. Scanner อ่าน byte แรกของไฟล์
2. ถ้าขึ้นต้นด้วย "---" → เข้าโหมด frontmatter
3. อ่านจนเจอ "---" ปิดท้าย (หรือ EOF → error)
4. ส่ง YAML string → parser (serde_yaml ใน Rust runtime)
5. Validate ตาม schema ที่กำหนด
6. Export เป็น frontmatter object
7. ตัด frontmatter block ออกจากเนื้อหา MDX
```

### ฟิลด์ทั้งหมดที่รองรับ

| ฟิลด์            | ชนิด                | Required | Default     | คำอธิบาย                                                |
| ---------------- | ------------------- | -------- | ----------- | ------------------------------------------------------- |
| `title`          | `string`            | ❌       | `undefined` | หัวข้อหน้า ใช้เป็น `<title>`                            |
| `description`    | `string`            | ❌       | `undefined` | คำอธิบายย่อ ใช้ meta description                        |
| `publishedAt`    | `string` (ISO 8601) | ❌       | `undefined` | วันที่เผยแพร่                                           |
| `updatedAt`      | `string` (ISO 8601) | ❌       | `undefined` | วันที่แก้ไขล่าสุด                                       |
| `author.name`    | `string`            | ❌       | `undefined` | ชื่อผู้เขียน                                            |
| `author.avatar`  | `string`            | ❌       | `undefined` | URL รูปผู้เขียน                                         |
| `author.bio`     | `string`            | ❌       | `undefined` | ประวัติย่อผู้เขียน                                      |
| `author.twitter` | `string`            | ❌       | `undefined` | Twitter/X handle                                        |
| `tags`           | `string[]`          | ❌       | `[]`        | แท็กสำหรับจัดหมวดหมู่                                   |
| `image`          | `string`            | ❌       | `undefined` | OG image path (ใช้ใน social share)                      |
| `draft`          | `boolean`           | ❌       | `false`     | `true` = ข้ามตอน build (ใช้กับ SSG)                     |
| `noindex`        | `boolean`           | ❌       | `false`     | `true` = เพิ่ม `<meta name="robots" content="noindex">` |
| `canonical`      | `string` (URL)      | ❌       | `undefined` | canonical URL                                           |
| `layout`         | `string`            | ❌       | `undefined` | ระบุ layout พิเศษ                                       |

### Validation Rules จาก Rust

| กฎ                                                      | เงื่อนไข                       | Error Code |
| ------------------------------------------------------- | ------------------------------ | ---------- |
| YAML ต้อง parse ได้                                     | syntax error → RUV1205         | RUV1205    |
| `draft` ต้องเป็น boolean                                | string "true" → error          | RUV1205    |
| `publishedAt` ต้องเป็น ISO 8601                         | รูปแบบไม่ถูกต้อง → error       | RUV1205    |
| `tags` ต้องเป็น array                                   | string → error                 | RUV1205    |
| `image` path ห้ามเป็น absolute URL (ถ้าไม่ใช่ external) | `https://...` ใน local → เตือน | RUV1205    |
| `canonical` ต้องเป็น valid URL                          | URL parse ไม่ผ่าน → error      | RUV1205    |
| `noindex` ต้องเป็น boolean                              | number → error                 | RUV1205    |

### ตัวอย่าง Frontmatter เต็มรูปแบบ

```mdx
---
title: เจาะลึก Ruvyxa MDX Pipeline
description:
  เรียนรู้ทุกขั้นตอนการทำงานของ MDX compiler ใน Ruvyxa ตั้งแต่ frontmatter ไปจนถึง image
  optimization
publishedAt: 2026-07-29T09:00:00.000Z
updatedAt: 2026-07-30T14:30:00.000Z
author:
  name: นักพัฒนาไทย
  avatar: /images/authors/dev-thai.webp
  bio: Full-stack developer, Ruvyxa core team
  twitter: ruvyxa_dev
tags:
  - ruvyxa
  - mdx
  - pipeline
  - thai
  - advanced
image: /images/blog/og-mdx-pipeline.jpg
draft: false
noindex: false
canonical: https://ruvyxa.dev/guides/deep-dive/mdx-pipeline
layout: docs
customField:
  nested:
    key: value
---
```

### การเข้าถึง Frontmatter ใน Code

```tsx
// app/blog/page.tsx
import { frontmatter } from './[slug]/page.mdx'
import { Link } from '@ruvyxa/react'

interface PostListItem {
  slug: string
  title: string
  description: string
  publishedAt: string
  tags: string[]
}

export default function BlogIndexPage() {
  const posts: PostListItem[] = [
    {
      slug: 'mdx-pipeline',
      title: 'เจาะลึก MDX Pipeline',
      description: 'เรียนรู้ทุกขั้นตอนการทำงานของ MDX compiler',
      publishedAt: '2026-07-29',
      tags: ['mdx', 'pipeline'],
    },
    {
      slug: 'image-optimization',
      title: 'Image Optimization ใน Ruvyxa',
      description: 'วิธีปรับรูปภาพให้เร็วและเล็กลง',
      publishedAt: '2026-07-30',
      tags: ['image', 'performance'],
    },
  ]

  return (
    <div className="blog-list">
      {posts.map((post) => (
        <Link key={post.slug} href={`/blog/${post.slug}`}>
          <article className="blog-card">
            <h2>{post.title}</h2>
            <p>{post.description}</p>
            <time dateTime={post.publishedAt}>
              {new Date(post.publishedAt).toLocaleDateString('th-TH', {
                year: 'numeric',
                month: 'long',
                day: 'numeric',
              })}
            </time>
          </article>
        </Link>
      ))}
    </div>
  )
}
```

---

## MDX Features

### JSX Components ใน MDX — ทุกกรณี

```mdx
---
title: Component Examples
---

import { useState } from 'react'
import Counter from '../../components/counter.tsx'
import Card from '../../components/card.tsx'
import Alert from '../../components/alert.tsx'

# ตัวอย่าง Components

{/* Server Component — ไม่ต้องมี 'use client' */}

<Card title="Ruvyxa">เนื้อหาที่เรนเดอร์จาก server</Card>

{/* Client Component — ต้องมี 'use client' ในไฟล์ component */}

<Counter initialValue={10} />

{/* Component พร้อม Props หลากหลาย */}

<Alert type="warning" dismissable={true} onDismiss={() => console.log('dismissed')}>
  คำเตือน: component นี้ทำงานทั้งสองฝั่ง
</Alert>
```

**กติกา:**

| Component Type     | `'use client'` | ใช้ `useState`/`useEffect` | เรนเดอร์ฝั่ง     |
| ------------------ | -------------- | -------------------------- | ---------------- |
| Server Component   | ไม่ต้อง        | ❌ ใช้ไม่ได้               | Server           |
| Client Component   | ต้องมี         | ✅                         | Client (hydrate) |
| Shared Component   | ไม่ต้อง        | ❌ ใช้ไม่ได้               | Server           |
| Third-party UI lib | ต้องมี         | ✅ (ปกติ)                  | Client           |

### Expressions (JSX ใน `{}`)

```mdx
# การคำนวณและการวนลูป

{/* คำนวณเลข */} ผลรวม: {1 + 2 + 3 + 4 + 5}

{/* Ternary */} {isLoggedIn ? 'ยินดีต้อนรับกลับ' : 'กรุณาเข้าสู่ระบบ'}

{/* Array mapping */}

<ul>
  {['แดง', 'เขียว', 'น้ำเงิน'].map((color, i) => (
    <li key={i} style={{ color }}>
      {color}
    </li>
  ))}
</ul>

{/* วันที่ */}

<p>
  วันนี้:{' '}
  {new Date().toLocaleDateString('th-TH', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    weekday: 'long',
  })}
</p>

{/* Logical operators */} {error && <Alert type="error">{error}</Alert>}

{count === 0 && <p>ไม่มีรายการ</p>}

{/* Template literals */}

<p>{`สวัสดี ${username} คุณมี ${notifications.length} การแจ้งเตือน`}</p>
```

### ESM Imports — ทุกประเภท

```mdx
import { formatDistanceToNow } from 'date-fns'
import { Card } from './components/index'
import data from './data.json'
import styles from './styles.module.css'
import Image from 'next/image'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'

# ESM Import Examples

{/* Import React components */}

<Card title="Hello" />

{/* Import JSON */}

<p>{data.name}</p>

{/* Import CSS modules */}

<div className={styles.container}>...</div>

{/\*\*

- ข้อจำกัด:
- - dynamic import() ไม่รองรับใน MDX context
- - imports ต้องอยู่ต้นไฟล์ (ก่อน content)
- - path ต้องถูกต้องเมื่อเทียบกับตำแหน่งไฟล์ MDX \*/}
```

**กฎ ESM Imports ใน MDX:**

1. ต้องอยู่ด้านบนสุดของไฟล์ (ก่อน Markdown content)
2. `import` statements ระหว่าง frontmatter และเนื้อหาเท่านั้น
3. ห้ามใช้ `require()` — ใช้ ESM syntax เท่านั้น
4. Dynamic `import()` ไม่รองรับ (ใช้ใน component แทน)
5. Relative imports ต้องสัมพันธ์กับตำแหน่งไฟล์ `.page.mdx`

---

## Auto-Exports จาก MDX — ทุก Export

### `frontmatter`

```ts
export const frontmatter: MDXFrontmatter = {
  title: 'หน้าแรก',
  description: 'คำอธิบาย',
  publishedAt: '2026-07-29',
  tags: ['guide'],
  // ... fields ทั้งหมดจาก YAML
}
```

### `meta`

```ts
export const meta: RouteMeta = {
  path: '/blog/mdx-pipeline',
  slug: 'mdx-pipeline',
  params: { slug: 'mdx-pipeline' },
  title: 'เจาะลึก MDX Pipeline | บล็อก',
  description: 'เรียนรู้ทุกขั้นตอนการทำงานของ MDX compiler',
}
```

| ฟิลด์         | ชนิด                     | ที่มา                             |
| ------------- | ------------------------ | --------------------------------- |
| `path`        | `string`                 | เส้นทางเต็มของ route              |
| `slug`        | `string`                 | slug จาก URL param หรือ filename  |
| `params`      | `Record<string, string>` | dynamic params จาก route          |
| `title`       | `string`                 | `frontmatter.title` + site suffix |
| `description` | `string`                 | `frontmatter.description`         |

### `headings`

```ts
export const headings: Heading[] = [
  { depth: 1, text: 'หน้าแรก', id: 'หน้าแรก' },
  { depth: 2, text: 'บทนำ', id: 'บทนำ' },
  { depth: 3, text: 'การติดตั้ง', id: 'การติดตั้ง' },
  { depth: 2, text: 'วิธีใช้', id: 'วิธีใช้' },
  { depth: 3, text: 'CLI Commands', id: 'cli-commands' },
  { depth: 4, text: 'ruvyxa dev', id: 'ruvyxa-dev' },
]
```

| ฟิลด์   | ชนิด                         | ที่มา                                 |
| ------- | ---------------------------- | ------------------------------------- |
| `depth` | `1 \| 2 \| 3 \| 4 \| 5 \| 6` | ระดับ heading (`#` = 1, `##` = 2 ฯลฯ) |
| `text`  | `string`                     | ข้อความใน heading                     |
| `id`    | `string`                     | slugified id สำหรับ anchor link       |

### `contentFormat`

```ts
export const contentFormat: 'mdx' | 'markdown' = 'mdx'
```

| ค่า          | ไฟล์        |
| ------------ | ----------- |
| `'mdx'`      | `.page.mdx` |
| `'markdown'` | `.md`       |

### Auto-Exports Reference Table

| Export          | ชนิด                    | มีใน `.page.mdx` | มีใน `.md` |
| --------------- | ----------------------- | ---------------- | ---------- |
| `frontmatter`   | `MDXFrontmatter`        | ✅               | ✅         |
| `meta`          | `RouteMeta`             | ✅               | ❌         |
| `headings`      | `Heading[]`             | ✅               | ❌         |
| `contentFormat` | `'mdx' \| 'markdown'`   | ✅               | ✅         |
| `content`       | `string` (raw markdown) | ❌               | ✅         |

### ตัวอย่าง: สร้าง Table of Contents แบบ Recursive

```tsx
// components/toc.tsx
import { headings } from '../app/docs/page.mdx'

interface TocItem {
  depth: number
  text: string
  id: string
  children: TocItem[]
}

function buildTocTree(headings: Heading[]): TocItem[] {
  const root: TocItem[] = []
  const stack: TocItem[] = []

  for (const h of headings) {
    const item: TocItem = { ...h, children: [] }

    // Pop stack จนกว่าจะเจอ parent ที่ depth น้อยกว่า
    while (stack.length > 0 && stack[stack.length - 1].depth >= h.depth) {
      stack.pop()
    }

    if (stack.length === 0) {
      root.push(item)
    } else {
      stack[stack.length - 1].children.push(item)
    }

    stack.push(item)
  }

  return root
}

function TocList({ items }: { items: TocItem[] }) {
  if (items.length === 0) return null

  return (
    <ul>
      {items.map((item) => (
        <li key={item.id}>
          <a href={`#${item.id}`}>{item.text}</a>
          <TocList items={item.children} />
        </li>
      ))}
    </ul>
  )
}

export default function TableOfContents() {
  const tree = buildTocTree(headings)
  return <TocList items={tree} />
}
```

---

## MDX ESM Deduplication — กลไกภายใน

เมื่อมีการ import MDX ไฟล์เดียวกันหลายครั้ง (เช่น import frontmatter จากหลาย component) Ruvyxa ใช้
**module deduplication** เพื่อไม่ต้อง compile ซ้ำ:

```ts
// Internal: MDX Module Cache
interface MDXCachedModule {
  source: string // compiled JS output
  exports: {
    frontmatter: Record<string, unknown>
    meta: RouteMeta
    headings: Heading[]
    contentFormat: 'mdx' | 'markdown'
  }
  dependencies: string[] // list of imported modules
  contentHash: string // blake3 hash of source
  compiledAt: number // timestamp
}

// 512-entry LRU Cache
class LRUCache<K, V> {
  private capacity: number
  private cache: Map<K, V>

  constructor(capacity: number) {
    this.capacity = capacity
    this.cache = new Map()
  }

  get(key: K): V | undefined {
    if (!this.cache.has(key)) return undefined
    const value = this.cache.get(key)!
    // Move to tail (most recently used)
    this.cache.delete(key)
    this.cache.set(key, value)
    return value
  }

  set(key: K, value: V): void {
    if (this.cache.has(key)) {
      this.cache.delete(key)
    } else if (this.cache.size >= this.capacity) {
      // Evict least recently used
      const lruKey = this.cache.keys().next().value
      if (lruKey !== undefined) {
        this.cache.delete(lruKey)
      }
    }
    this.cache.set(key, value)
  }

  has(key: K): boolean {
    return this.cache.has(key)
  }
}

// Cache key = realpath + content hash
function getMDXCacheKey(filePath: string, contentHash: string): string {
  return `${filePath}::${contentHash}`
}

// Deduplication entry point
function compileMDXIfNeeded(filePath: string): MDXCachedModule {
  const realPath = fs.realpathSync(filePath)
  const contentHash = hashFile(realPath) // blake3-256
  const cacheKey = getMDXCacheKey(realPath, contentHash)

  // Cache hit — skip compilation
  if (MDX_CACHE.has(cacheKey)) {
    return MDX_CACHE.get(cacheKey)!
  }

  // Cache miss — compile
  const compiled = compileMDXFile(realPath)

  // Validate dependencies haven't changed
  const depHashes = compiled.dependencies.map((dep) => hashFile(dep))
  compiled.exports = { ...compiled.exports, _depHashes: depHashes }

  MDX_CACHE.set(cacheKey, compiled)
  return compiled
}

const MDX_CACHE = new LRUCache<string, MDXCachedModule>(512)
```

### Cache Invalidation Rules

| Event                        | Action                                       |
| ---------------------------- | -------------------------------------------- |
| MDX source เปลี่ยน           | contentHash เปลี่ยน → miss cache → recompile |
| Dependency (import) เปลี่ยน  | dep hash ตรวจไม่ตรง → recompile              |
| `ruvyxa clean`               | ลบ cache ทั้งหมด + disk cache                |
| Dev server restart           | cache persist ไปยัง disk -> ยังใช้ได้        |
| Config เปลี่ยน (image, etc.) | cache entries ที่เกี่ยวข้องถูก invalidate    |
| Cache เต็ม (512 entries)     | LRU eviction — เอาตัวที่ไม่ได้ใช้นานสุดออก   |

---

## AST Node Lowering — การแปลง Internal Nodes

Ruvyxa แปลง (lower) โหนด AST พิเศษให้เป็น HTML หรือ components ที่เหมาะสม

### Tables

```mdx
| Header 1 | Header 2 | Header 3 |
| -------- | :------: | -------: |
| Left     |  Center  |    Right |
| Normal   | **bold** |   `code` |
```

↓

```html
<div class="table-wrapper">
  <table>
    <thead>
      <tr>
        <th>Header 1</th>
        <th style="text-align: center">Header 2</th>
        <th style="text-align: right">Header 3</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>Left</td>
        <td style="text-align: center">Center</td>
        <td style="text-align: right">Right</td>
      </tr>
      <tr>
        <td>Normal</td>
        <td style="text-align: center"><strong>bold</strong></td>
        <td style="text-align: right"><code>code</code></td>
      </tr>
    </tbody>
  </table>
</div>
```

### Code Blocks — Syntax Highlighting

````mdx
```rust
fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
```
````

↓ (ใช้ Prism หรือ Shiki syntax highlighter)

```html
<pre class="language-rust">
  <code class="language-rust">
    <span class="token keyword">fn</span>
    <span class="token function">fibonacci</span>
    <span class="token punctuation">(</span>
    <span class="token parameter">n</span>
    <span class="token operator">:</span>
    <span class="token builtin">u32</span>
    <span class="token punctuation">)</span>
    <span class="token punctuation">-></span>
    <span class="token builtin">u32</span>
    <span class="token punctuation">{</span>
    ...
  </code>
</pre>
```

### Images (Markdown → Image Component)

```mdx
![คำอธิบาย](/images/photo.jpg)
```

↓

```tsx
<Image
  src="/images/photo.jpg"
  alt="คำอธิบาย"
  width={1200}
  height={800} // read from image metadata
  loading="lazy"
/>
```

**Special case:** ถ้า Ruvyxa อ่าน metadata ของรูปไม่ได้ (ไฟล์ไม่มีขนาด) จะใช้ fallback:

```tsx
<Image
  src="/images/photo.jpg"
  alt="คำอธิบาย"
  // width/height ไม่ระบุ → browser จะคำนวณเอง
  // อาจเกิด Cumulative Layout Shift
/>
```

### Task Lists (Checkboxes)

```mdx
- [x] งานที่เสร็จแล้ว
- [ ] งานที่ยังไม่เสร็จ
- [x] งานที่เสร็จแล้ว 2
```

↓

```html
<ul class="task-list">
  <li class="task-list-item">
    <input type="checkbox" id="task-0" checked disabled />
    <label for="task-0">งานที่เสร็จแล้ว</label>
  </li>
  <li class="task-list-item">
    <input type="checkbox" id="task-1" disabled />
    <label for="task-1">งานที่ยังไม่เสร็จ</label>
  </li>
  <li class="task-list-item">
    <input type="checkbox" id="task-2" checked disabled />
    <label for="task-2">งานที่เสร็จแล้ว 2</label>
  </li>
</ul>
```

### Math (LaTeX)

```mdx
สมการของไอน์สไตน์: $E = mc^2$

สูตรสมการกำลังสอง:

$$
x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}
$$

อินทิกรัล:

$$
\int_{0}^{\infty} e^{-x^2} \, dx = \frac{\sqrt{\pi}}{2}
$$
```

↓

```html
<p>
  สมการของไอน์สไตน์:
  <span class="math math-inline">E = mc<sup>2</sup></span>
</p>

<p>สูตรสมการกำลังสอง:</p>
<div class="math math-display" aria-label="x = (-b ± sqrt(b² - 4ac)) / (2a)">
  x = (-b ± sqrt(b² - 4ac)) / (2a)
</div>

<p>อินทิกรัล:</p>
<div class="math math-display">∫₀^∞ e^(-x²) dx = sqrt(π) / 2</div>
```

### Footnotes

```mdx
Ruvyxa เป็นเฟรมเวิร์ก React สำหรับคนไทย[^1]

[^1]: Ruvyxa พัฒนาด้วย Rust + TypeScript เพื่อประสิทธิภาพสูงสุด
```

↓

```html
<p>
  Ruvyxa เป็นเฟรมเวิร์ก React สำหรับคนไทย
  <sup class="footnote-ref">
    <a href="#fn1" id="fnref1">[1]</a>
  </sup>
</p>

<section class="footnotes">
  <h2>เชิงอรรถ</h2>
  <ol>
    <li id="fn1">
      Ruvyxa พัฒนาด้วย Rust + TypeScript เพื่อประสิทธิภาพสูงสุด
      <a href="#fnref1" class="footnote-backref">↩︎</a>
    </li>
  </ol>
</section>
```

---

## Content Cache — 512-entry LRU

### Lifecycle Diagram

```
MDX Source File
    │
    ▼
Compute blake3-256(content)
    │
    ▼
Build cache key: realpath + hash
    │
    ▼
┌─── LRU Cache Lookup ───┐
│                         │
│  HIT? ──yes──→ Return   │
│  (skip compile)  cached │
│           module        │
│  no                    │
│    ↓                    │
│  Compile MDX            │
│    ↓                    │
│  Compute dep hashes     │
│    ↓                    │
│  Store in cache         │
│  (evict LRU if full)   │
└─────────────────────────┘
```

### Cache Statistics (Monitoring)

```ts
interface MDXCacheStats {
  size: number // current entries
  maxSize: number // 512
  hits: number // total cache hits
  misses: number // total cache misses
  hitRate: number // hits / (hits + misses)
  evictions: number // times LRU evicted
  avgCompileTime: number // milliseconds
}
```

---

## Image Component (`@ruvyxa/react`) — Full Reference

### Complete Props Type Definition

```ts
import { ImgHTMLAttributes } from 'react'

interface ImageProps extends Omit<
  ImgHTMLAttributes<HTMLImageElement>,
  'src' | 'alt' | 'width' | 'height' | 'loading' | 'srcSet' | 'sizes' | 'style'
> {
  // Required
  src: string
  alt: string

  // Dimensions
  width?: number
  height?: number

  // Loading behavior
  priority?: boolean
  loading?: 'lazy' | 'eager'
  lazy?: boolean

  // Quality and format
  quality?: number // 1-100
  format?: 'webp' | 'avif' | 'auto' // default: auto

  // Responsive
  sizes?: string // CSS sizes media query
  srcSet?: never // auto-generated, cannot override

  // Styling
  objectFit?: 'cover' | 'contain' | 'fill' | 'none' | 'scale-down'
  objectPosition?: string // CSS object-position
  className?: string
  style?: React.CSSProperties

  // Placeholder
  placeholder?: 'blur' | 'empty'
  blurDataURL?: string // base64-encoded blur image

  // Debug
  onLoad?: (event: React.SyntheticEvent<HTMLImageElement>) => void
  onError?: (event: React.SyntheticEvent<HTMLImageElement>) => void
}
```

### Props Reference Table

| Prop             | ชนิด                         | Required | Default   | คำอธิบาย                                  |
| ---------------- | ---------------------------- | -------- | --------- | ----------------------------------------- |
| `src`            | `string`                     | ✅       | -         | Path หรือ URL ของรูป                      |
| `alt`            | `string`                     | ✅       | -         | ข้อความ alternative (accessibility)       |
| `width`          | `number`                     | ❌       | auto      | ความกว้างต้นฉบับ (px)                     |
| `height`         | `number`                     | ❌       | auto      | ความสูงต้นฉบับ (px)                       |
| `priority`       | `boolean`                    | ❌       | `false`   | Preload + eager loading (LCP)             |
| `loading`        | `'lazy' \| 'eager'`          | ❌       | `'lazy'`  | วิธีโหลด                                  |
| `lazy`           | `boolean`                    | ❌       | `true`    | เปิด/ปิด lazy loading                     |
| `quality`        | `number`                     | ❌       | `80`      | คุณภาพ (1-100, สูง = ไฟล์ใหญ่)            |
| `format`         | `'webp' \| 'avif' \| 'auto'` | ❌       | `'auto'`  | รูปแบบ output                             |
| `sizes`          | `string`                     | ❌       | -         | CSS sizes attribute                       |
| `objectFit`      | `string`                     | ❌       | -         | CSS object-fit                            |
| `objectPosition` | `string`                     | ❌       | -         | CSS object-position                       |
| `className`      | `string`                     | ❌       | -         | CSS class name                            |
| `style`          | `React.CSSProperties`        | ❌       | -         | Inline styles                             |
| `placeholder`    | `'blur' \| 'empty'`          | ❌       | `'empty'` | แสดง placeholder ขณะโหลด                  |
| `blurDataURL`    | `string`                     | ❌       | -         | Base64 blur (ใช้กับ `placeholder='blur'`) |
| `onLoad`         | `function`                   | ❌       | -         | Callback เมื่อโหลดเสร็จ                   |
| `onError`        | `function`                   | ❌       | -         | Callback เมื่อโหลด error                  |

### Priority Loading (LCP Optimization)

```tsx
// components/hero.tsx
import { Image } from '@ruvyxa/react'

export default function HeroBanner() {
  return (
    <section className="hero">
      <Image
        src="/images/home/hero-banner.jpg"
        alt="Ruvyxa Framework — สร้างเว็บไทยด้วย React"
        width={1920}
        height={1080}
        priority
        quality={85}
        format="avif"
        sizes="100vw"
      />
      <div className="hero-overlay">
        <h1>Ruvyxa</h1>
        <p>เฟรมเวิร์ก React สำหรับคนไทย</p>
      </div>
    </section>
  )
}
```

**Ruvyxa เจอ `priority` = `true` แล้วทำ:**

1. เพิ่ม `<link rel="preload" as="image" href="/assets/hero.abc123.avif">` ใน `<head>`
2. สำหรับทุกรูปแบบที่ตั้งไว้ (WebP + AVIF) — สร้าง preload links ทั้งหมด
3. ตั้ง `loading="eager"` อัตโนมัติ
4. เพิ่ม `fetchpriority="high"` attribute
5. จัดลำดับความสำคัญใน build pipeline ให้ประมวลผลก่อนรูปอื่น

[!TIP] ใช้ `priority` เฉพาะรูปที่อยู่เหนือ fold (above-the-fold) เท่านั้น — ประมาณ 1-3 รูปต่อหน้า

### Lazy Loading (Default)

รูปอื่น ๆ ทั้งหมดใช้ IntersectionObserver API:

```tsx
// components/gallery.tsx
import { Image } from '@ruvyxa/react'

interface GalleryImage {
  src: string
  alt: string
  width: number
  height: number
}

export default function PhotoGallery({ images }: { images: GalleryImage[] }) {
  return (
    <div className="gallery-grid">
      {images.map((img, index) => (
        <div key={index} className="gallery-item">
          <Image
            src={img.src}
            alt={img.alt}
            width={img.width}
            height={img.height}
            loading={index < 2 ? 'eager' : 'lazy'}
            objectFit="cover"
            sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
          />
        </div>
      ))}
    </div>
  )
}
```

### Sizes + Srcset — Responsive Images

```tsx
<Image
  src="/images/product/main.jpg"
  alt="รูปสินค้า"
  width={800}
  height={600}
  sizes="
    (max-width: 480px) 100vw,
    (max-width: 768px) 50vw,
    (max-width: 1200px) 33vw,
    25vw
  "
/>
```

**Output HTML:**

```html
<img
  alt="รูปสินค้า"
  src="/assets/main.abc123.webp"
  srcset="
    /assets/main.abc123-640w.webp   640w,
    /assets/main.abc123-960w.webp   960w,
    /assets/main.abc123-1280w.webp 1280w,
    /assets/main.abc123-1920w.webp 1920w,
    /assets/main.abc123-2560w.webp 2560w
  "
  sizes="(max-width: 480px) 100vw, (max-width: 768px) 50vw, (max-width: 1200px) 33vw, 25vw"
  loading="lazy"
  width="800"
  height="600"
/>
```

[!NOTE] Browser เลือก variant ที่เหมาะสมที่สุดโดยพิจารณาจาก:

- ความละเอียดหน้าจอ (DPR)
- viewport width
- ขนาดที่ระบุใน `sizes`

### Placeholder Blur

```tsx
import { Image } from '@ruvyxa/react'

export default function BlurExample() {
  return (
    <div className="article-image">
      <Image
        src="/images/blog/cover.jpg"
        alt="ภาพปกบทความ"
        width={1200}
        height={600}
        placeholder="blur"
        blurDataURL="data:image/webp;base64,UklGRnoCAABXRUJQVlA4WAoAAAA..."
      />
    </div>
  )
}
```

Ruvyxa สร้าง `blurDataURL` อัตโนมัติตอน build:

- ถ้าไม่ระบุ → Ruvyxa สร้าง base64 8x8 px version ให้
- ถ้าระบุ → ใช้ที่ระบุ (ต้องเป็น data URL)

### External URL Images

```tsx
<Image
  src="https://cdn.example.com/photos/sunset.webp"
  alt="พระอาทิตย์ตก"
  width={1920}
  height={1080}
/>
```

**ข้อจำกัดรูปจาก external URL:**

| คุณสมบัติ            | Local (`public/`) | External (CDN)    |
| -------------------- | ----------------- | ----------------- |
| WebP/AVIF conversion | ✅                | ❌ (คง original)  |
| Responsive sizes     | ✅                | ❌                |
| Cache hash (blake3)  | ✅                | ❌                |
| Optimization         | ✅                | ❌                |
| Placeholder blur     | ✅                | ❌                |
| การทำงาน             | full pipeline     | ข้าม optimization |

**แนะนำ:** ใช้ CDN image service (Cloudinary, imgix, Cloudflare Images) สำหรับ external optimization

---

## Image Optimization (Build-Time) — Full Pipeline

### Pipeline Diagram

```
Build Start
    │
    ▼
┌──────────────────────────────────────────┐
│ Phase 1: Discovery                       │
│  • glob public/images/**/*.{jpg,png,gif} │
│  • scan imports/references ใน source     │
│  • filter out SVG, animated GIF          │
│  • group by source file                  │
└────────────────┬─────────────────────────┘
                 ▼
┌──────────────────────────────────────────┐
│ Phase 2: Decode & Metadata               │
│  • sharp().metadata()                    │
│  • strip EXIF orientation                │
│  • auto-rotate โดยใช้ EXIF               │
│  • compute entropy score                 │
└────────────────┬─────────────────────────┘
                 ▼
┌──────────────────────────────────────────┐
│ Phase 3: Resize                          │
│  • for each size in config.sizes[]       │
│  • sharp().resize(width, fit='outside')  │
│  • lanczos3 kernel                       │
│  • preserve aspect ratio                 │
└────────────────┬─────────────────────────┘
                 ▼
┌──────────────────────────────────────────┐
│ Phase 4: Encode                          │
│  ┌──────────────┬────────────┬──────────┐│
│  │ Original     │ Encoder    │ Config   ││
│  ├──────────────┼────────────┼──────────┤│
│  │ JPEG → WebP │ cwebp      │ q=80     ││
│  │ JPEG → AVIF │ libaom-av1 │ q=65     ││
│  │ JPEG → JPEG │ mozjpeg    │ q=80,p   ││
│  │ PNG  → WebP │ cwebp      │ q=80     ││
│  │ PNG  → AVIF │ libaom-av1 │ q=65     ││
│  │ PNG  → PNG  │ oxipng     │ o=3      ││
│  │ JPEG premium│ guetzli    │ q=85     ││
│  └──────────────┴────────────┴──────────┘│
└────────────────┬─────────────────────────┘
                 ▼
┌──────────────────────────────────────────┐
│ Phase 5: Hash & Write                    │
│  • blake3-256(content)                   │
│  • filename: {name}.{hash8}-{size}w.webp │
│  • write → .ruvyxa/assets/images/        │
│  • generate manifest.json                │
│  • generate placeholder blur             │
└──────────────────────────────────────────┘
```

### Encoder Parameters (Rust)

#### mozjpeg (JPEG → JPEG)

```rust
struct MozJPEGParams {
    quality: u8,           // 0-100, default: 80
    progressive: bool,     // default: true
    optimize_coding: bool, // default: true
    smooth: u8,           // 0-100, default: 0
    dct_method: DCTMethod, // Integer | Float
    trellis_quant: bool,   // default: true
    trellis_pass: bool,    // default: true
    overshoot_deringing: bool, // default: true
}
```

#### oxipng (PNG → PNG) — Lossless

```rust
struct OxiPNGParams {
    level: u8,             // 0-6, default: 3
    interlace: bool,       // default: false
    strip: StripMeta,      // Safe | All | None
    alpha: AlphaHandling,  // Preserve | Remove | Unpremultiply
    deflate: DeflateAlgo,  // Zlib | Zopfli
}
```

| Level | การทำงาน                     |
| ----- | ---------------------------- |
| 0     | no optimization              |
| 1     | basic filter + zlib          |
| 2     | + row filter trials          |
| 3     | + exhaustive filter trials   |
| 4     | + zopfli (ช้า)               |
| 5     | + full zopfli                |
| 6     | maximum compression (ช้ามาก) |

#### guetzli (JPEG — Premium Quality)

```rust
struct GuetzliParams {
    quality: u8,           // 84-100, default: 85
    memory_limit: usize,   // MB, default: 1024
}
```

[!WARNING] Guetzli ใช้ RAM สูง (~200MB+ ต่อรูป) และช้ามาก (10-30x ของ mozjpeg)
ใช้เฉพาะรูปสำคัญที่ต้องการคุณภาพสูงสุด

#### cwebp (WebP Encoder)

```rust
struct WebPParams {
    quality: u8,           // 0-100, default: 80
    method: u8,            // 0-6, default: 4 (0=เร็ว, 6=ช้าที่สุด)
    alpha_q: u8,           // 0-100, alpha compression
    pass: u8,              // 1-10, analysis pass
    preprocess: u8,        // 0 | 1 | 2
    filter_strength: u8,   // 0-100
    filter_type: u8,       // 0 | 1
}
```

#### libaom (AVIF Encoder)

```rust
struct AVIFParams {
    quality: u8,           // 0-100, default: 65
    speed: u8,             // 0-10, default: 6 (0=ช้าที่สุด)
    tile_rows: u8,         // default: 1
    tile_cols: u8,         // default: 1
    chroma_subsampling: Subsampling,  // YUV420 | YUV444
}
```

### Input/Output Matrix

| Input Format   | mozjpeg | oxipng        | guetzli      | cwebp            | libaom           | Copy           |
| -------------- | ------- | ------------- | ------------ | ---------------- | ---------------- | -------------- |
| JPEG           | ✅      | ❌            | ✅ (premium) | ✅               | ✅               | ❌             |
| PNG            | ❌      | ✅ (lossless) | ❌           | ✅               | ✅               | ❌             |
| GIF (static)   | ❌      | ❌            | ❌           | ✅               | ✅               | ❌             |
| GIF (animated) | ❌      | ❌            | ❌           | ❌               | ❌               | ✅             |
| SVG            | ❌      | ❌            | ❌           | ❌               | ❌               | ✅ (sanitized) |
| WebP           | ❌      | ❌            | ❌           | ✅ (re-compress) | ❌               | ✅             |
| AVIF           | ❌      | ❌            | ❌           | ❌               | ✅ (re-compress) | ✅             |
| BMP            | ✅      | ❌            | ❌           | ✅               | ✅               | ❌             |
| TIFF           | ✅      | ❌            | ❌           | ✅               | ✅               | ❌             |

### Output Hashed Filename Pattern

```
Source: public/images/blog/hero.jpg

.ruvyxa/assets/images/
├── hero.a1b2c3d4.webp               // original size
├── hero.a1b2c3d4.avif
├── hero.a1b2c3d4-640w.webp
├── hero.a1b2c3d4-640w.avif
├── hero.a1b2c3d4-960w.webp
├── hero.a1b2c3d4-960w.avif
├── hero.a1b2c3d4-1280w.webp
├── hero.a1b2c3d4-1280w.avif
├── hero.a1b2c3d4-1920w.webp
├── hero.a1b2c3d4-1920w.avif
├── hero.a1b2c3d4-2560w.webp
├── hero.a1b2c3d4-2560w.avif
├── placeholder/
│   └── hero.a1b2c3d4-blur.webp
└── manifest.json
```

### Image Config — Every Field

```ts
// ruvyxa.config.ts — image config
import { defineConfig } from 'ruvyxa/config'

export default defineConfig({
  image: {
    formats: ['webp', 'avif'], // รูปแบบ output
    sizes: [640, 960, 1280, 1920, 2560], // responsive widths
    quality: 85, // WebP quality (1-100)
    avifQuality: 70, // AVIF quality (1-100)
    lazy: true, // default lazy loading
    placeholder: 'blur', // 'blur' | 'empty'
    encoder: {
      jpeg: 'mozjpeg', // mozjpeg | guetzli | libjpeg
      png: 'oxipng', // oxipng | pngquant | libpng
      jpegQuality: 80, // mozjpeg/libjpeg quality
      pngQuality: 85, // pngquant quality (lossy)
      pngCompressionLevel: 3, // oxipng level (0-6)
    },
    cache: {
      enabled: true,
      directory: '.ruvyxa/cache/images',
    },
  },
})
```

| Field                         | TypeScript Type                       | Default                  | Validation                   |
| ----------------------------- | ------------------------------------- | ------------------------ | ---------------------------- |
| `formats`                     | `('webp' \| 'avif')[]`                | `['webp', 'avif']`       | ต้องมีอย่างน้อย 1 รายการ     |
| `sizes`                       | `number[]`                            | `[640, 1280, 1920]`      | แต่ละค่าต้อง > 0 และ < 10000 |
| `quality`                     | `number`                              | `80`                     | 1-100                        |
| `avifQuality`                 | `number`                              | `65`                     | 1-100                        |
| `lazy`                        | `boolean`                             | `true`                   | -                            |
| `placeholder`                 | `'blur' \| 'empty'`                   | `'empty'`                | -                            |
| `encoder.jpeg`                | `'mozjpeg' \| 'guetzli' \| 'libjpeg'` | `'mozjpeg'`              | -                            |
| `encoder.png`                 | `'oxipng' \| 'pngquant' \| 'libpng'`  | `'oxipng'`               | -                            |
| `encoder.jpegQuality`         | `number`                              | `80`                     | 1-100                        |
| `encoder.pngQuality`          | `number`                              | `85`                     | 1-100 (pngquant only)        |
| `encoder.pngCompressionLevel` | `number`                              | `3`                      | 0-6 (oxipng only)            |
| `cache.enabled`               | `boolean`                             | `true`                   | -                            |
| `cache.directory`             | `string`                              | `'.ruvyxa/cache/images'` | must be relative             |

---

## SEO Metadata Components — Full Reference

### `<Meta>` Component — Props

```ts
interface MetaProps {
  // Basic
  title?: string
  description?: string

  // Open Graph
  openGraph?: {
    title?: string
    description?: string
    image?: string | OGImage
    url?: string
    type?: 'website' | 'article' | 'profile' | 'book' | 'music.song' | 'video.movie'
    siteName?: string
    locale?: string // th_TH, en_US ฯลฯ
    alternateLocale?: string[]
  }

  // Twitter Card
  twitter?: {
    card?: 'summary' | 'summary_large_image' | 'app' | 'player'
    site?: string // @username
    creator?: string // @username
    image?: string
  }

  // Robots & Canonical
  noindex?: boolean
  nofollow?: boolean
  canonical?: string

  // Additional
  keywords?: string[] // <meta name="keywords">
  jsonLd?: Record<string, unknown> // JSON-LD inline
}
```

### `<Seo>` Component — Props

```ts
interface SeoProps {
  title?: string
  description?: string
  canonical?: string
  noindex?: boolean
  jsonLd?: Record<string, unknown> | Record<string, unknown>[]
}
```

### JSON-LD — Article

```tsx
import { Seo } from '@ruvyxa/react'

interface BlogPostFrontmatter {
  title: string
  description: string
  publishedAt: string
  updatedAt?: string
  author?: { name: string; avatar?: string; bio?: string }
  image?: string
}

export default function BlogPost({ frontmatter }: { frontmatter: BlogPostFrontmatter }) {
  const articleLd: Record<string, unknown> = {
    '@context': 'https://schema.org',
    '@type': 'Article',
    headline: frontmatter.title,
    description: frontmatter.description,
    datePublished: frontmatter.publishedAt,
    dateModified: frontmatter.updatedAt || frontmatter.publishedAt,
    author: {
      '@type': 'Person',
      name: frontmatter.author?.name || 'ผู้เขียน',
      image: frontmatter.author?.avatar,
      description: frontmatter.author?.bio,
    },
    image: frontmatter.image ? `${siteUrl}${frontmatter.image}` : undefined,
    publisher: {
      '@type': 'Organization',
      name: 'Ruvyxa Blog',
      logo: {
        '@type': 'ImageObject',
        url: 'https://ruvyxa.dev/logo.png',
      },
    },
    mainEntityOfPage: {
      '@type': 'WebPage',
      '@id': `https://ruvyxa.dev/blog/${frontmatter.slug}`,
    },
  }

  return (
    <>
      <Seo
        title={`${frontmatter.title} | บล็อก`}
        description={frontmatter.description}
        canonical={`https://ruvyxa.dev/blog/${frontmatter.slug}`}
        jsonLd={articleLd}
      />
      <article>{/* เนื้อหา */}</article>
    </>
  )
}
```

### JSON-LD — BreadcrumbList

```tsx
import { Seo } from '@ruvyxa/react'

export default function ProductPage() {
  const breadcrumbLd = {
    '@context': 'https://schema.org',
    '@type': 'BreadcrumbList',
    itemListElement: [
      { '@type': 'ListItem', position: 1, name: 'หน้าแรก', item: 'https://example.com' },
      { '@type': 'ListItem', position: 2, name: 'สินค้า', item: 'https://example.com/products' },
      {
        '@type': 'ListItem',
        position: 3,
        name: 'สินค้า A',
        item: 'https://example.com/products/a',
      },
    ],
  }

  return <Seo jsonLd={breadcrumbLd} />
}
```

### JSON-LD — Organization

```tsx
const organizationLd = {
  '@context': 'https://schema.org',
  '@type': 'Organization',
  name: 'Ruvyxa',
  url: 'https://ruvyxa.dev',
  logo: 'https://ruvyxa.dev/logo.png',
  description: 'เฟรมเวิร์ก React สัญชาติไทย',
  foundingDate: '2025',
  founders: [{ '@type': 'Person', name: 'ผู้ก่อตั้ง' }],
  contactPoint: {
    '@type': 'ContactPoint',
    contactType: 'customer support',
    email: 'support@ruvyxa.dev',
  },
  sameAs: ['https://github.com/ruvyxa', 'https://twitter.com/ruvyxa'],
}

export default function AboutPage() {
  return <Seo title="เกี่ยวกับ Ruvyxa" jsonLd={organizationLd} />
}
```

### JSON-LD — FAQPage

```tsx
const faqLd = {
  '@context': 'https://schema.org',
  '@type': 'FAQPage',
  mainEntity: [
    {
      '@type': 'Question',
      name: 'Ruvyxa ใช้ภาษาอะไรพัฒนา?',
      acceptedAnswer: {
        '@type': 'Answer',
        text: 'Ruvyxa พัฒนาด้วย Rust และ TypeScript',
      },
    },
    {
      '@type': 'Question',
      name: 'Ruvyxa รองรับ MDX หรือไม่?',
      acceptedAnswer: {
        '@type': 'Answer',
        text: 'รองรับ MDX เต็มรูปแบบ พร้อม image optimization และ SEO',
      },
    },
  ],
}
```

### SEO Tags — Mapping Table

| Ruvyxa Config/Frontmatter | HTML Tag ที่สร้าง                                                                                    |
| ------------------------- | ---------------------------------------------------------------------------------------------------- |
| `title`                   | `<title>`, `<meta property="og:title">`, `<meta name="twitter:title">`                               |
| `description`             | `<meta name="description">`, `<meta property="og:description">`, `<meta name="twitter:description">` |
| `image`                   | `<meta property="og:image">`, `<meta name="twitter:image">`                                          |
| `canonical`               | `<link rel="canonical">`                                                                             |
| `noindex`                 | `<meta name="robots" content="noindex">`                                                             |
| `nofollow`                | `<meta name="robots" content="nofollow">`                                                            |
| `openGraph.type`          | `<meta property="og:type">`                                                                          |
| `openGraph.locale`        | `<meta property="og:locale">`                                                                        |
| `openGraph.siteName`      | `<meta property="og:site_name">`                                                                     |
| `twitter.card`            | `<meta name="twitter:card">`                                                                         |
| `twitter.site`            | `<meta name="twitter:site">`                                                                         |
| `twitter.creator`         | `<meta name="twitter:creator">`                                                                      |
| `keywords`                | `<meta name="keywords">`                                                                             |
| `jsonLd`                  | `<script type="application/ld+json">`                                                                |

---

## เต็มรูปแบบ: ตัวอย่างโปรเจกต์ MDX

### 1. โครงสร้างไฟล์

```
my-blog/
├── app/
│   ├── layout.tsx
│   ├── page.tsx
│   ├── page.mdx                 # หน้าแรกแบบ MDX
│   └── blog/
│       ├── page.tsx              # หน้ารวมบทความ
│       └── [slug]/
│           └── page.mdx          # แต่ละบทความ
├── public/
│   └── images/
│       ├── blog/
│       │   ├── hero.jpg
│       │   └── og-default.jpg
│       └── authors/
│           └── avatar.webp
├── components/
│   ├── counter.tsx
│   ├── alert.tsx
│   └── toc.tsx
├── ruvyxa.config.ts
├── package.json
└── .env
```

### 2. Root Layout

```tsx
// app/layout.tsx
import { Meta, Link } from '@ruvyxa/react'
import type { ReactNode } from 'react'

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="th">
      <head>
        <Meta
          title="Ruvyxa Blog"
          description="บล็อกภาษาไทยเกี่ยวกับ Ruvyxa"
          openGraph={{
            title: 'Ruvyxa Blog',
            description: 'บล็อกภาษาไทยเกี่ยวกับ Ruvyxa',
            image: '/images/blog/og-default.jpg',
            type: 'website',
            locale: 'th_TH',
          }}
          twitter={{
            card: 'summary_large_image',
          }}
        />
        <link rel="alternate" type="application/rss+xml" title="Ruvyxa Blog" href="/feed.xml" />
      </head>
      <body>
        <nav>
          <Link href="/">หน้าแรก</Link>
          <Link href="/blog">บล็อก</Link>
          <Link href="/about">เกี่ยวกับ</Link>
        </nav>
        <main>{children}</main>
        <footer>© {new Date().getFullYear()} Ruvyxa Blog</footer>
      </body>
    </html>
  )
}
```

### 3. Blog Page — MDX + Image + SEO

````mdx
---
title: วิธีใช้ Ruvyxa สำหรับมือใหม่
description: คู่มือเริ่มต้นใช้งาน Ruvyxa สำหรับนักพัฒนาไทย
publishedAt: 2026-07-29T09:00:00Z
updatedAt: 2026-07-30T14:30:00Z
author:
  name: นักพัฒนาไทย
  avatar: /images/authors/avatar.webp
  bio: Full-stack developer และสมาชิกทีม Ruvyxa
  twitter: ruvyxa_dev
tags: [ruvyxa, guide, beginner, thai]
image: /images/blog/og-ruvyxa-guide.jpg
draft: false
---

import { Image, Meta } from '@ruvyxa/react'
import { headings } from './page.mdx'
import Alert from '../../../components/alert.tsx'
import Counter from '../../../components/counter.tsx'

<Meta
  title="{frontmatter.title} | บล็อก"
  description={frontmatter.description}
  openGraph={{
    title: frontmatter.title,
    description: frontmatter.description,
    image: frontmatter.image,
    type: 'article',
  }}
/>

# {frontmatter.title}

<time dateTime={frontmatter.publishedAt}>
  {new Date(frontmatter.publishedAt).toLocaleDateString('th-TH', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })}
</time>
| โดย <strong>{frontmatter.author.name}</strong>| แท็ก: {frontmatter.tags.join(', ')}

<Image
  src={frontmatter.image}
  alt="ภาพปก — Ruvyxa Framework"
  width={1200}
  height={630}
  priority
  quality={85}
  format="avif"
/>

<nav class="toc">
  <h2>📑 สารบัญ</h2>
  <ul>
    {headings
      .filter((h) => h.depth === 2)
      .map((h) => (
        <li key={h.id}>
          <a href={`#${h.id}`}>{h.text}</a>
        </li>
      ))}
  </ul>
</nav>

## บทนำ

Ruvyxa เป็นเฟรมเวิร์ก React สำหรับคนไทย...

<Alert type="info">บทความนี้เหมาะสำหรับผู้เริ่มต้นที่เคยใช้ React มาก่อน</Alert>

## วิธีติดตั้ง

<Image
  src="/images/blog/install-steps.png"
  alt="ขั้นตอนการติดตั้ง Ruvyxa"
  width={800}
  height={400}
  loading="lazy"
/>

```bash
npm create ruvyxa@latest my-app
cd my-app
npm run dev
```
````

## Interactive Component

<Counter initialValue={10} />

## สรุป

Ruvyxa ทำให้การพัฒนาเว็บด้วย React ง่ายขึ้น...

<Alert type="success">
  ติดตั้งเสร็จแล้ว! ลองสร้างหน้า MDX หน้าแรกของคุณดู
</Alert>
```

### 4. รายการ Blog — Import Frontmatter

```tsx
// app/blog/page.tsx
import { Meta, Link } from '@ruvyxa/react'

interface PostMeta {
  slug: string
  title: string
  description: string
  publishedAt: string
  author: string
  tags: string[]
  image: string
}

const posts: PostMeta[] = [
  {
    slug: 'getting-started',
    title: 'วิธีใช้ Ruvyxa สำหรับมือใหม่',
    description: 'คู่มือเริ่มต้นใช้งาน Ruvyxa',
    publishedAt: '2026-07-29',
    author: 'นักพัฒนาไทย',
    tags: ['guide', 'beginner'],
    image: '/images/blog/thumb-getting-started.webp',
  },
  {
    slug: 'mdx-deep-dive',
    title: 'เจาะลึก MDX Pipeline',
    description: 'ทุกขั้นตอนการทำงานของ MDX compiler',
    publishedAt: '2026-07-30',
    author: 'นักพัฒนาไทย',
    tags: ['mdx', 'advanced'],
    image: '/images/blog/thumb-mdx.webp',
  },
]

export default function BlogIndexPage() {
  return (
    <>
      <Meta title="บล็อก | Ruvyxa" description="รวมบทความภาษาไทย" />
      <h1>บล็อก</h1>
      <div class="blog-grid">
        {posts.map((post) => (
          <Link key={post.slug} href={`/blog/${post.slug}`}>
            <article class="blog-card">
              <h2>{post.title}</h2>
              <p>{post.description}</p>
              <time>{post.publishedAt}</time>
              <div class="tags">
                {post.tags.map((tag) => (
                  <span key={tag} class="tag">
                    {tag}
                  </span>
                ))}
              </div>
            </article>
          </Link>
        ))}
      </div>
    </>
  )
}
```

### 5. Config

```ts
// ruvyxa.config.ts
import { defineConfig } from 'ruvyxa/config'

export default defineConfig({
  site: {
    url: 'https://blog.ruvyxa.dev',
    sitemap: {
      exclude: ['/draft/*'],
      defaults: {
        changeFrequency: 'weekly',
        priority: 0.5,
      },
    },
  },
  image: {
    formats: ['webp', 'avif'],
    sizes: [640, 960, 1280, 1920],
    quality: 85,
    avifQuality: 70,
    lazy: true,
    placeholder: 'blur',
  },
  css: {
    entries: ['src/styles/blog.css'],
  },
})
```

---

## การใช้ `<img>` vs `<Image>` Component

| ความสามารถ              | `<img>`           | `<Image>`         |
| ----------------------- | ----------------- | ----------------- |
| Build-time optimization | ✅ (บางส่วน)      | ✅ (เต็มรูปแบบ)   |
| WebP/AVIF conversion    | ❌                | ✅                |
| Responsive srcset       | ❌                | ✅                |
| Lazy loading            | ต้องเพิ่มเอง      | ✅ อัตโนมัติ      |
| Priority preload        | ❌                | ✅                |
| Placeholder blur        | ❌                | ✅                |
| Cache hash              | ❌ (ใช้ original) | ✅ (blake3)       |
| Object fit              | CSS class         | ✅ prop           |
| Accessibility           | ✅ (alt)          | ✅ (alt required) |

---

## Analyze — ตรวจสอบการใช้งานรูป

```
npm run analyze
```

Output:

```
━━━ Analyze ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Image Optimization Report
  ─────────────────────────
  Total images:            24
  Using <Image>:           22  (91.7%)
  Using <img>:              2  (8.3%)   ⚠️
  Unoptimized external:     2  (CDN URLs)

  Format Distribution
  ────────────────────
  WebP:                   24 variants
  AVIF:                   22 variants
  JPEG (original):         4 (external only)

  Size Savings
  ─────────────
  Original total:        3,920 KB
  Optimized total:       1,490 KB
  Saved:                 2,430 KB  (62%)

  Encoder Performance
  ───────────────────
  mozjpeg:       12 files  avg 58% compression
  oxipng:         8 files  avg 45% compression
  cwebp:         24 files  avg 65% compression
  libaom:        22 files  avg 70% compression

  Recommendations
  ───────────────
  ⚠️ 2 images still use <img> — switch to <Image>
  ⚠️ Hero images missing priority — add priority prop
  ✅ All local images have WebP variants
```

---

## Troubleshooting — ทุก Error Code

| Error Code | ปัญหา                  | สาเหตุ                            | วิธีแก้                     |
| ---------- | ---------------------- | --------------------------------- | --------------------------- |
| RUV1005    | Missing Meta component | หน้าไม่มี SEO metadata            | เพิ่ม `<Meta>` หรือ `<Seo>` |
| RUV1010    | Module not found       | import path ผิด                   | ตรวจ path สัมพัทธ์          |
| RUV1201    | MDX compile error      | JSX syntax ผิด                    | ตรวจ JSX ใน MDX             |
| RUV1202    | Image not found        | src path ไม่มีไฟล์                | ตรวจ public/images/         |
| RUV1203    | Image not optimized    | ใช้ `<img>`                       | เปลี่ยนเป็น `<Image>`       |
| RUV1204    | LCP image no priority  | รูป hero ไม่มี priority           | เพิ่ม `priority` prop       |
| RUV1205    | Frontmatter invalid    | YAML parse error                  | ตรวจ `---` และ syntax       |
| RUV1206    | Language not supported | Syntax highlight ภาษาที่ไม่รองรับ | เปลี่ยนเป็นภาษามาตรฐาน      |

| ปัญหาทั่วไป                  | สาเหตุ                    | วิธีแก้                                     |
| ---------------------------- | ------------------------- | ------------------------------------------- |
| MDX ไม่แสดง JSX component    | ขาด `'use client'`        | เพิ่ม `'use client'` directive ใน component |
| Frontmatter export หาย       | YAML syntax error         | ตรวจ `---` ปิด/เปิด                         |
| Headings export ว่าง         | ไม่มี heading ใน MDX      | เพิ่ม `#` headings                          |
| Image ไม่แสดงผล              | src path ผิด              | ตรวจ public/ directory                      |
| Image external ไม่มี variant | External ไม่ผ่าน pipeline | ใช้ local หรือ CDN service                  |
| LCP performance ต่ำ          | รูปไม่มี priority         | เพิ่ม `priority`                            |
| JSON-LD ไม่ทำงาน             | schema.org ข้อมูลผิด      | ตรวจ @context, @type                        |
| MDX compile ช้า              | 512-entry cache เต็ม      | LRU eviction อัตโนมัติ หรือ `ruvyxa clean`  |
| Syntax highlight ไม่มีสี     | ภาษาที่ระบุผิด            | `tsx`, `js`, `rust`, `python`               |
| Table overflow               | ตารางกว้างเกิน            | ใช้ `table-wrapper` CSS                     |
| Task list checkbox ไม่แสดง   | CSS missing               | เพิ่ม `task-list` CSS                       |

---

## Logs and Debugging

```bash
# ดู MDX compilation logs
RUVYXA_DEBUG=mdx ruvyxa dev

# ดู image optimization logs
RUVYXA_DEBUG=image ruvyxa build

# ดู cache hit/miss
RUVYXA_DEBUG=cache ruvyxa dev

# Full debug
RUVYXA_DEBUG=* ruvyxa dev
```

---

## ลองทำดู

1. **MDX พื้นฐาน**
   - สร้าง `app/hello/page.mdx` พร้อม frontmatter
   - เพิ่ม `##` headings หลายระดับ
   - import `headings` สร้าง Table of Contents

2. **JSX Components**
   - สร้าง `components/alert.tsx` ( client)
   - import และใช้ใน MDX พร้อม props

3. **Image Optimization**
   - วางรูป JPEG 2MB+ ใน `public/images/`
   - ใช้ `<Image>` พร้อม `sizes` และ `priority`
   - `npm run build` → ดูขนาดใน `.ruvyxa/assets/images/`

4. **SEO**
   - เพิ่ม `<Meta>` component ใน MDX
   - ใช้ `<Seo>` พร้อม JSON-LD (Article)
   - ตรวจสอบ `<head>` output

5. **ตรวจสอบ**
   - `npm run analyze` — ดู image report
   - `npm run trace /hello` — ดู route manifest
   - Developer Tools → Network → ดู WebP/AVIF

---

## สรุป

- `page.mdx` = Markdown + JSX components + ESM imports
- MDX pipeline: frontmatter parse → remark/rehype compile → AST lowering → auto-export injection →
  LRU cache
- Auto-exports: `frontmatter`, `meta`, `headings[]`, `contentFormat`
- 512-entry LRU cache พร้อม blake3 content hashing
- AST node lowering: tables, code blocks, task lists, math, footnotes, images
- <Image> component: optimization, responsive srcset, lazy/priority, blur placeholder
- Build-time encoders: mozjpeg, oxipng, guetzli, cwebp, libaom (AVIF)
- SEO: <Meta> (OG, Twitter), <Seo> (JSON-LD: Article, BreadcrumbList, Organization, FAQ)
- ตรวจสอบ: `ruvyxa analyze`, `run trace`
