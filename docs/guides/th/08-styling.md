# การจัดแต่งสไตล์: CSS, SCSS, และ CSS Modules

Ruvyxa รองรับหลายวิธีในการแต่งสไตล์: Global CSS, SCSS/Sass (compiler grass), CSS Modules (พร้อม
fnv1a_64 hashing), CSS-in-JS (style objects), Tailwind CSS integration, และ css.entries สำหรับ
stylesheet นอก project

---

## ภาพรวมระบบ

```
┌──────────────────────────────────────────────────┐
│           วิธีแต่งสไตล์ใน Ruvyxa                 │
│                                                  │
│  Global CSS     — import ใน layout.tsx           │
│  SCSS/Sass      — .scss หรือ .sass              │
│  CSS Modules    — .module.css / .module.scss     │
│                   .module.sass                   │
│  Style objects  — React style={{}}              │
│  css.entries    — config สำหรับ global css       │
│  External CSS   — import จาก node_modules        │
│  Tailwind CSS   — @import "tailwindcss"          │
└──────────────────────────────────────────────────┘
```

---

## Type Definitions

```ts
// ruvyxa.config.ts
export interface CssConfig {
  /** ไฟล์ Global CSS ที่จะรวมอยู่ในทุกหน้า */
  entries?: string[]
  /** การตั้งค่า CSS Modules */
  modules?: {
    localsConvention?: 'camelCase' | 'camelCaseOnly' | 'dashes' | 'dashesOnly'
    generateScopedName?: string | ((name: string, filename: string, css: string) => string)
  }
  /** เปิดใช้งาน Tailwind CSS (ค่าเริ่มต้น: true ถ้ามี tailwind.config.js) */
  tailwind?: boolean
  /** การตั้งค่า Autoprefixer */
  autoprefixer?: boolean | object
  /** การตั้งค่า SCSS/Sass */
  preprocessorOptions?: {
    scss?: object
    sass?: object
  }
}
```

---

## Global CSS

import ไฟล์ CSS ใน root layout — CSS จะเป็น global, ใช้ได้กับทุก component โดยไม่ต้อง import ซ้ำ

### การใช้งานพื้นฐาน

```tsx
// app/layout.tsx
import './globals.css'

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="th">
      <body>{children}</body>
    </html>
  )
}
```

```css
/* app/globals.css */
:root {
  color-scheme: light;
  font-family: 'Sarabun', 'Noto Sans Thai', sans-serif;
  color: #17191f;
  background: #f6f7f2;
}

*,
*::before,
*::after {
  box-sizing: border-box;
}

body {
  margin: 0;
  min-height: 100vh;
}

a {
  color: #0b5cad;
  text-underline-offset: 3px;
}
a:hover {
  color: #083d7a;
}

code {
  background: #eceee8;
  border: 1px solid #dde2d8;
  border-radius: 5px;
  padding: 2px 6px;
  font-family: 'SFMono-Regular', Consolas, monospace;
}
```

### Path Resolution (Under the Hood)

Ruvyxa ใช้ dependency-driven style collection:

1. **Application seeds:** Walker ที่ scan ทุกไฟล์ใน `app/` (`.ts`, `.tsx`, `.js`, `.jsx`, `.mts`,
   `.cts`, `.mjs`, `.cjs`)
2. **Import graph traversal:** สำหรับแต่ละไฟล์ script, parse imports → ถ้า import CSS → เก็บเป็น
   seed → ถ้า import ไฟล์ script อื่น → push เข้า queue
3. **@import resolution:** สำหรับ CSS ที่เก็บได้, parse `@import` → resolve path → recursive

**File extensions ที่รองรับ:**

| Extension | Type | หมายเหตุ                      |
| --------- | ---- | ----------------------------- |
| `.css`    | CSS  | plain CSS                     |
| `.scss`   | SCSS | ต้อง compile ด้วย grass       |
| `.sass`   | Sass | indent-based syntax           |
| `.less`   | Less | ❌ ไม่รองรับ (ต้องใช้ plugin) |

**ไม่รองรับ Less/Stylus/etc. โดยตรง** — ถ้าต้องการ ต้อง compile เป็น CSS ก่อน (RUV1402)

---

## SCSS / Sass

Ruvyxa รองรับ SCSS/Sass โดยตรงโดยใช้ compiler **grass** ในตัว — ไม่ต้อง setup webpack loader

### ตัวอย่าง

```scss
// app/styles/variables.scss
$primary: #0d766e;
$secondary: #f5b84b;
$text: #17191f;
$radius: 8px;
$font-thai: 'Sarabun', 'Noto Sans Thai', sans-serif;
```

```scss
// app/styles/mixins.scss
@mixin card {
  border: 1px solid #dfe5dc;
  border-radius: $radius;
  background: #fff;
  padding: 22px;
  box-shadow: 0 1px 2px rgba(23, 25, 31, 0.04);
}

@mixin responsive($breakpoint) {
  @media (max-width: $breakpoint) {
    @content;
  }
}
```

```scss
// app/styles/global.scss
@use './variables' as *;
@use './mixins' as *;

.product-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
  margin: 32px 0;
}

.product-card {
  @include card;
  transition: transform 0.15s ease;

  &:hover {
    transform: translateY(-2px);
  }

  h3 {
    margin: 0 0 8px;
    color: $text;
    font-family: $font-thai;
  }
}
```

import ใน layout:

```tsx
import './styles/global.scss'
```

### Under the Hood: grass Compiler Options

```rust
fn compile_sass_file(path: &Path, project_root: &Path) -> Result<String, String> {
    let options = grass::Options::default()
        .style(grass::OutputStyle::Expanded)    // output style = expanded
        .load_path(project_root)                 // resolve @use/@import จาก project root
        .load_path(project_root.join("node_modules")); // resolve จาก node_modules
    grass::from_path(path, &options)
}
```

- `OutputStyle::Expanded` — output CSS ไม่ minified (การ minify ทำแยกต่างหาก)
- `load_path` — ทั้ง `project_root` และ `node_modules` ทำให้ `@use 'package'` ทำงานได้
- ใช้ `grass::from_path()` ไม่ใช่ `from_string()` — เพื่อให้ source map references ถูกต้อง

### Under the Hood: Sass Import Resolution

Ruvyxa มี `resolve_sass_import()` สำหรับค้นหาไฟล์ Sass:

```rust
fn resolve_sass_import(root, base_dir, specifier) -> Option<PathBuf> {
    let base = if specifier.starts_with('.') {
        base_dir.join(specifier)                  // relative
    } else if specifier.starts_with('/') {
        root.join(specifier.trim_start_matches('/')) // absolute ใน project
    } else {
        root.join("node_modules").join(specifier)    // from node_modules
    };

    // Candidates (เรียงตาม priority)
    candidates = [
        base,                           // exact path
        base.scss,                      // + .scss
        base.sass,                      // + .sass
        _base.scss,                     // Sass partial
        _base.sass,
        base/index.scss,                // directory index
        base/_index.scss,
        base/index.sass,
        base/_index.sass,
    ];
}
```

### Sass Import Types ที่รองรับ

```scss
@use './variables'; // Sass partial → _variables.scss
@use './mixins' as *; // import without namespace
@use 'package'; // จาก node_modules
@forward './theme'; // re-export
@import './legacy'; // deprecated แต่ยังรองรับ
```

**Sass @import graph** ถูก traverse อย่างมีประสิทธิภาพ — shared partials (เช่น `_variables.scss`)
จะถูก walk เพียงครั้งเดียว ไม่ซ้ำต่อ entry

### หมายเหตุ

- **ห้ามใช้ `@import`** (deprecated ใน Sass) — ให้ใช้ `@use` แทน
- Ruvyxa รายงาน RUV1402 ถ้า Sass compilation ล้มเหลว
- ไฟล์ partial (`_filename.scss`) ถูก import โดย `@use './filename'` โดยไม่ต้องใช้ underscore

---

## CSS Modules

`.module.css`, `.module.scss`, หรือ `.module.sass` → class names ถูก scoped โดยอัตโนมัติด้วย
fnv1a_64 hash

### การใช้งาน

```css
/* app/components/Button.module.css */
.button {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  border: none;
  border-radius: 7px;
  background: #17191f;
  padding: 11px 16px;
  color: #fff;
  font-size: 14px;
  font-weight: 700;
  cursor: pointer;
}

.button:hover {
  background: #2d3140;
}

.primary {
  background: #0d766e;
}
.primary:hover {
  background: #0a5e58;
}

.large {
  padding: 14px 24px;
  font-size: 16px;
}
```

```tsx
// app/components/Button.tsx
import styles from './Button.module.css'

interface ButtonProps {
  children: React.ReactNode
  variant?: 'default' | 'primary'
  size?: 'normal' | 'large'
  onClick?: () => void
}

export default function Button({
  children,
  variant = 'default',
  size = 'normal',
  onClick,
}: ButtonProps) {
  const className = [
    styles.button,
    variant === 'primary' ? styles.primary : '',
    size === 'large' ? styles.large : '',
  ]
    .filter(Boolean)
    .join(' ')

  return (
    <button className={className} onClick={onClick}>
      {children}
    </button>
  )
}
```

Output CSS จะเป็น:

```css
.Button_button_feff5ad3a1e67b7b {
}
.Button_primary_1a2b3c4d5e6f7890 {
}
.Button_large_abcdef1234567890 {
}
```

### Under the Hood: fnv1a_64 Algorithm

```rust
fn scoped_class_name(path: &Path, project_root: &Path, local: &str) -> String {
    let relative = normalized_relative_path(path, project_root);
    let digest = fnv1a_64(format!("{relative}:{local}").as_bytes());
    let stem = path.file_stem()       // "Button.module"
        .trim_end_matches(".module")  // "Button"
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();

    format!("{stem}_{local}__{digest:016x}")
    // เช่น "Button_button__feff5ad3a1e67b7b"
}
```

fnv1a_64 implementation:

```rust
fn fnv1a_64(input: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;  // FNV offset basis
    for byte in input {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    hash
}
```

**คุณสมบัติของ class name ที่สร้าง:**

| Component      | ค่า | คำอธิบาย                                       |
| -------------- | --- | ---------------------------------------------- |
| Deterministic  | ✅  | class name เดิมเสมอสำหรับ path+class name เดิม |
| Scoped by path | ✅  | component ต่าง directory → class name ต่างกัน  |
| 16 hex digits  | ✅  | 64-bit hash → collision probability ~1/2^64    |
| Human-readable | ✅  | prefix เป็นชื่อไฟล์ + local class name         |

**Golden value test:**

```
path: /project/styles/card.module.css
class: .card
hash: fnv1a_64("styles/card.module.css:card")
result: card_card__feff5ad3a1e67b7b
```

### CSS Module Detection

```rust
pub fn is_css_module_path(path: &Path) -> bool {
    let name = path.file_name().to_ascii_lowercase();
    name.ends_with(".module.css")
        || name.ends_with(".module.scss")
        || name.ends_with(".module.sass")
}
```

ไฟล์ที่ไม่ลงท้ายด้วย `.module.css`/`.module.scss`/`.module.sass` จะถูก treat เป็น global CSS ไม่ถูก
scope

### composes

CSS Modules รองรับ `composes` — reuse classes จาก module เดียวกัน (ไม่รองรับ
`composes: ... from 'file'` ข้ามไฟล์):

```css
/* app/components/Card.module.css */
.card {
  border: 1px solid #dfe5dc;
  border-radius: 8px;
  background: #fff;
  padding: 22px;
}

.highlight {
  composes: card;
  border-color: #f5b84b;
  box-shadow: 0 0 0 2px rgba(245, 184, 75, 0.3);
}

.dark {
  composes: card;
  background: #1f2030;
  color: #eef2ff;
  border-color: #303247;
}
```

**Semantics:** `composes: card;` ทำให้ class `highlight` มี class name ของ `card` รวมอยู่ด้วย:

```css
.card_card__feff5ad3a1e67b7b {
  /* ... */
}
.highlight_card__xxxxxxxx {
  /* ... */
}
```

เมื่อใช้ `styles.highlight` → className จะเป็น
`"highlight_card__xxxxxxxx card_card__feff5ad3a1e67b7b"`

**Under the hood:** `local_composition()` parser จะตรวจสอบ keyword `composes:` หลัง `{` แล้วแยก
class names ที่ตามมา:

```rust
fn local_composition(chars, start) -> Option<(usize, Vec<String>)> {
    // 1. ตรวจสอบคำว่า "composes"
    // 2. whitespace → ":" → class names → ";"
    // 3. return list of class names to compose
}
```

### :global() Escape

ใช้ `:global()` เพื่อ escape จาก CSS Modules scope — สำหรับ定义 global styles หรือ keyframes:

```css
/* card.module.css */
.card :global(.highlight-text) {
  color: #f5b84b;
}

/* global keyframes */
@keyframes :global(fadeIn) {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.animated {
  animation: fadeIn 0.3s ease;
}

/* global class selector */
:global(.theme-dark) .card {
  background: #1f2030;
}
```

`::global()` ถูกประมวลผลโดย `global_selector_contents()` ซึ่งจับคู่ `(` `)` และคืน content
ภายในโดยไม่ถูก scope

### CSS Module vs Global CSS

| คุณสมบัติ    | CSS Module                    | Global CSS        |
| ------------ | ----------------------------- | ----------------- |
| File pattern | `*.module.css`                | `*.css`           |
| Scoping      | ✅ auto-scoped                | ❌ global         |
| Class name   | `{stem}_{class}__{hash}`      | ตามที่เขียน       |
| Import       | `import styles from './file'` | `import './file'` |
| `composes`   | ✅ รองรับ                     | ❌ ไม่รองรับ      |
| `:global()`  | ✅ escape ได้                 | ไม่จำเป็น         |
| Production   | ✅ reproducible               | ✅ reproducible   |

---

## HMR (Hot Module Replacement)

Ruvyxa รองรับ Hot Module Replacement สำหรับ CSS ทุกแบบ:

| วิธี                           | HMR                 | ประเภท                                      |
| ------------------------------ | ------------------- | ------------------------------------------- |
| Global CSS (.css)              | ✅ instant hot swap | CSS ถูกแทนที่ inline โดยไม่ต้อง reload หน้า |
| SCSS (.scss)                   | ✅ instant hot swap | Compile → CSS → hot swap                    |
| CSS Modules (.module.css)      | ✅ instant hot swap | Scope → replace                             |
| SCSS Modules (.module.scss)    | ✅ instant hot swap | Compile → scope → replace                   |
| import ใน component (.tsx/.ts) | ✅ instant hot swap | CSS changed → hot update                    |
| style objects                  | ❌ ต้อง full reload | inline style ไม่มี hot path                 |

**กลไก HMR:** Ruvyxa ใช้ `hmr_tracker` ที่ watch ไฟล์ใน `app/` และ directories ที่เกี่ยวข้อง เมื่อ
CSS เปลี่ยน:

1. ตรวจจับการเปลี่ยนแปลง (file watcher)
2. compile ใหม่ (ถ้าเป็น SCSS)
3. scope ใหม่ (ถ้าเป็น CSS Module)
4. ส่ง hot update ผ่าน WebSocket (`/__ruvyxa/hmr`)
5. browser รับ event → แทนที่ `<style>` element โดยไม่ reload

---

## css.entries

สำหรับ CSS ที่ไม่ได้ import ใน component code — ใช้ `css.entries` ใน config:

```tsx
// ruvyxa.config.ts
import { config, type RuvyxaConfig } from 'ruvyxa/config'

const settings: RuvyxaConfig = {
  css: {
    entries: [
      './styles/global.scss', // ไฟล์เดี่ยว
      './node_modules/some-lib/dist/style.css', // จาก node_modules
      './shared-styles/', // directory (scan ทั้งหมด)
    ],
  },
}

export default config(settings)
```

### ใช้เมื่อ:

- มี styles directory นอก `app/`
- ใช้ CSS จาก npm package
- ต้องการ compile หลายไฟล์เป็น entry เดียว
- ใช้ Tailwind CSS

### Under the Hood: Entry Resolution

```rust
fn collect_explicit_entry(root, entry, styles) {
    let entry = if absolute { entry } else { root.join(entry) };

    // 1. ตรวจสอบว่าอยู่ใน project root → RUV1404 ถ้าเกิน
    // 2. ถ้าเป็น directory → walk หา .css/.scss/.sass files
    // 3. ถ้าเป็น file → ตรวจสอบ extension → Append
    // 4. ไม่เจอ → RUV1403
}
```

**RUV1404:** CSS entry ต้องอยู่ใน project root — ป้องกันการอ้างอิงไฟล์นอก project โดยไม่ตั้งใจ

---

## External CSS Imports

import CSS จาก npm packages โดยตรง:

```tsx
// app/layout.tsx
import 'prismjs/themes/prism-tomorrow.css'
import '@fontsource/sarabun/400.css'
import '@fontsource/sarabun/700.css'
```

หรือใช้ css.entries:

```tsx
css: {
  entries: [
    '@fontsource/sarabun/400.css',
    '@fontsource/sarabun/700.css',
    'prismjs/themes/prism-tomorrow.css',
  ],
}
```

### Resolution Order

เมื่อ import CSS specifier ที่ไม่ขึ้นต้นด้วย `.` หรือ `/`:

1. ตรวจสอบ `tsconfig.json` paths → ถ้าเจอ → ใช้ค่านั้น
2. ตรวจสอบ `root/specifier` → ถ้าเป็นไฟล์ → ใช้
3. ตรวจสอบ `node_modules/specifier` → fallback

---

## CSS Ordering

ลำดับของการ Inject CSS ไปที่เบราว์เซอร์มีความสำคัญ (เพื่อหลีกเลี่ยงปัญหา CSS Specificity) Ruvyxa
ใช้ลำดับดังนี้:

1. `css.entries` จาก `ruvyxa.config.ts` (Global reset/utilities)
2. `import './global.css'` ใน `app/layout.tsx` (Root layouts)
3. CSS ที่ import เข้าไปใน Server Components
4. CSS ที่ import เข้าไปใน Client Components (โหลดแบบ Async หรือ Dynamic)

---

## Style Objects (CSS-in-JS)

React style objects — ไม่ต้อง import ไฟล์ CSS, เขียน inline ได้เลย:

```tsx
export default function Highlight() {
  const style = {
    background: 'linear-gradient(135deg, #0d766e, #0a5e58)',
    color: 'white',
    padding: '24px',
    borderRadius: '8px',
    fontFamily: "'Sarabun', sans-serif",
  }

  return (
    <div style={style}>
      <h2>ข้อความเน้น</h2>
      <p>นี่คือ style object ใน React</p>
    </div>
  )
}
```

หรือ inline:

```tsx
<button
  style={{
    background: '#17191f',
    color: '#fff',
    border: 'none',
    borderRadius: 7,
    padding: '11px 16px',
    cursor: 'pointer',
  }}
>
  คลิก
</button>
```

---

## Tailwind CSS Integration

Ruvyxa รองรับ Tailwind CSS ผ่านการ import `@import "tailwindcss"` ในไฟล์ CSS:

```css
/* app/globals.css */
@import 'tailwindcss';
```

เมื่อ Ruvyxa เจอ `@import "tailwindcss"` ใน stylesheet:

1. ค้นหา `@tailwindcss/cli` binary ใน `node_modules/.bin/`
2. ถ้าไม่พบ → RUV1401 (ติดตั้งด้วย `pnpm add tailwindcss && pnpm add -D @tailwindcss/cli`)
3. รัน CLI: `tailwindcss -i <input> --minify`
4. ใช้ output แทนที่ CSS นั้น
5. ถ้า CLI error → RUV1400

```tsx
// ruvyxa.config.ts (ไม่จำเป็น ถ้าใช้ @import)
css: {
  entries: ['./app/globals.css'],
}
```

---

## CSS Minification

Ruvyxa ใช้ LightningCSS (เขียนด้วย Rust) สำหรับ Minify CSS และจัดการ Vendor Prefixes อัตโนมัติ
ซึ่งเร็วโคตรๆ:

- ลบ Whitespace/Comments
- แปลงสีเป็นรูปแบบที่สั้นที่สุด (`#ff0000` -> `red`)
- ยุบรวม (Merge) Rules ที่เหมือนกัน
- ลดขนาดการเขียน CSS calc()

---

## Remote Style Imports

คุณสามารถดึง CSS จาก CDN ภายนอกมาใช้ในโปรเจกต์ได้โดยตรง (ถูกดาวน์โหลดตอน Build time):

```css
/* app/global.css */
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;700&display=swap');
@import 'https://cdn.jsdelivr.net/npm/normalize.css@8.0.1/normalize.css';
```

Ruvyxa จะดาวน์โหลดเนื้อหาของ `@import` ที่เป็น HTTP(s) และฝัง (inline) ไปในไฟล์ CSS สุดท้าย
(เพื่อประสิทธิภาพที่ดีขึ้น)

---

## Style End Tag Escaping

เมื่อแทรก Inline `<style>` ฝั่ง Server, เนื้อหา CSS อาจมีข้อมูลที่เบราว์เซอร์มองว่าเป็นปิดแท็ก
`</style>` ซึ่งทำให้เกิดช่องโหว่ XSS Ruvyxa Escape แท็กเหล่านี้ใน CSS ให้อัตโนมัติ:

```css
/* โค้ดต้นฉบับ */
.bad-content::after {
  content: "</style><script>alert('XSS')</script>";
}

/* ถูก Escape ให้อัตโนมัติเป็น */
.bad-content::after {
  content: "<\/style><script>alert('XSS')<\/script>";
}
```

---

## Error Codes Reference

| รหัสข้อผิดพลาด | คำอธิบาย                                  | วิธีแก้ไข                              |
| -------------- | ----------------------------------------- | -------------------------------------- |
| `RUV3001`      | หาไฟล์ CSS ที่ import ไม่เจอ              | ตรวจสอบ Path หรือการสะกดชื่อไฟล์       |
| `RUV3002`      | Syntax Error ในไฟล์ SCSS/CSS              | ตรวจสอบบรรทัดที่มีข้อผิดพลาดในไฟล์ CSS |
| `RUV3003`      | `css.entries` อ้างอิงไฟล์ที่ไม่มีอยู่จริง | ตรวจสอบ Path ใน `ruvyxa.config.ts`     |
| `RUV3004`      | Tailwind config มีข้อผิดพลาด              | ตรวจสอบไฟล์ `tailwind.config.js`       |

---

## Performance Characteristics

| ฟีเจอร์          | ความซับซ้อน | หมายเหตุ                                   |
| ---------------- | ----------- | ------------------------------------------ |
| CSS Minification | `O(n)`      | เร็วมาก (ใช้ LightningCSS ภายใน)           |
| SCSS Compilation | `O(n)`      | โหลด dart-sass ผ่าน worker thread          |
| Tailwind JIT     | `O(n)`      | สแกนไฟล์ที่ถูกเปลี่ยนเพื่อสร้างคลาสแบบ JIT |

---

## Edge Cases

### การใช้ `@import` ใน CSS Modules

CSS Modules รองรับการใช้ `@import` เพื่อดึงตัวแปร (variables) จากไฟล์อื่น
แต่ควรระวังเรื่องการใช้โค้ดซ้ำซ้อน แนะนำให้ใช้ SCSS `@use` แทนถ้าเป็นไปได้

### ลำดับของ Tailwind Utilities

ถ้าคุณใช้ Tailwind ร่วมกับ Custom CSS โปรดจำไว้ว่าคลาสของ Tailwind จะถูก Inject ตามลำดับในไฟล์หลัก
หากมีปัญหา Specificity ให้ใช้ `@layer` ของ Tailwind เข้าช่วย

---

## Under the Hood: Style Collection Pipeline

```
Application Source Files (app/)
    │
    ├── WalkDir scan → collect script files (.ts, .tsx, .js, .jsx, .mts, .cts, .mjs, .cjs)
    │
    ├── Traverse import graph
    │   ├── import './styles.css'    → seed style
    │   ├── import './component'     → queue script
    │   └── import 'package'         → ถ้าอยู่ใน project → queue
    │
    └── CSS Entries (from config)
        └── collect_explicit_entry()
            ├── directory → walk
            └── file → validate
                │
                ▼
            StyleWalk (shared state)
                │
                ├── visited: BTreeSet<PathBuf>    // dedup
                ├── file_index: BTreeSet<PathBuf>  // member index
                ├── sass_walked: BTreeSet<PathBuf> // partial dedup
                │
                ├── append_style(entry)
                │   ├── ถ้า visited → skip
                │   ├── read file
                │   ├── @import "tailwindcss"? → compile Tailwind
                │   ├── .scss/.sass? → compile_sass_file + collect_sass_dependencies
                │   ├── .module.css? → scope_css_module
                │   ├── parse @import → resolve → recursive
                │   ├── remove local @import statements
                │   ├── push CSS → output
                │   └── record file
                │
                └── output: StyleCollection { css, files }
```

### StyleCollection Structure

```rust
pub struct StyleCollection {
    pub css: String,            // CSS ที่รวมแล้ว
    pub files: Vec<PathBuf>,    // ไฟล์ที่ contribute
}
```

### CSS Minification

Ruvyxa มี minifier CSS ในตัวสำหรับ production (conservative — ไม่เปลี่ยน semantics):

```rust
pub fn minify_css(source: &str) -> String {
    let no_comments = strip_css_comments(source);   // ลบ /* ... */
    collapse_css_whitespace(&no_comments)            // ลบ whitespace ส่วนเกิน
}
```

**สิ่งที่ minifier ทำ:**

- ลบ block comments (`/* ... */`)
- แทนที่ whitespace runs ด้วย space เดียว
- ลบ space หน้า/หลัง punctuation (`{`, `}`, `:`, `;`, `,`, `(`, `)`)
- **ไม่ทำ:** shorthand merging, selector optimization, color minification (conservative)

**สิ่งที่ minifier ไม่ทำลาย:**

- String literals (`"..."`, `'...'`) — เก็บ verbatim
- `url()` values
- CSS `content` properties

### Remote @import Preservation

`@import` URLs ที่ remote (`http://`, `https://`, `//`, `data:`) จะถูกเก็บไว้不变 ไม่ถูก resolve:

```css
@import 'https://fonts.googleapis.com/css2?family=Sarabun';
@import '//cdn.example.com/theme.css';
```

จะคงอยู่ใน output CSS โดยไม่ถูกดึงหรือ resolve

### Security: </style> Escaping

เมื่อ CSS ถูกแทรกใน HTML `<style>` tag, Ruvyxa จะ escape `</style` → `<\/style` เพื่อป้องกัน
premature closing:

```rust
fn escape_style_end_tags(css: &str) -> String {
    // แทนที่ </style → <\/style (case-insensitive)
}
```

---

## ตัวอย่างเต็ม: ร้านค้า

```scss
// app/styles/variables.scss
$color-primary: #0d766e;
$color-primary-dark: #0a5e58;
$color-text: #17191f;
$color-bg: #f6f7f2;
$color-white: #ffffff;
$color-border: #dfe5dc;
$radius-sm: 6px;
$radius-md: 8px;
$radius-lg: 12px;
$font-body: 'Sarabun', 'Noto Sans Thai', sans-serif;
$font-mono: 'SFMono-Regular', Consolas, monospace;
$shadow-card: 0 1px 2px rgba(23, 25, 31, 0.04);
$shadow-hover: 0 10px 28px rgba(23, 25, 31, 0.1);
```

```scss
// app/styles/layout.scss
@use './variables' as *;

.store-container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 24px;
  font-family: $font-body;
}

.store-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 0;
  border-bottom: 1px solid $color-border;
}

.store-header__logo {
  font-size: 24px;
  font-weight: 800;
  color: $color-primary;
  text-decoration: none;
}

.store-header__nav {
  display: flex;
  gap: 16px;
}

.store-header__link {
  color: $color-text;
  text-decoration: none;
  font-weight: 600;
  &:hover {
    color: $color-primary;
  }
}
```

```css
/* app/styles/products.module.css */
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 20px;
  margin: 32px 0;
}

.card {
  border: 1px solid #dfe5dc;
  border-radius: 8px;
  background: #fff;
  padding: 22px;
  transition:
    transform 0.15s ease,
    box-shadow 0.15s ease;
}

.card:hover {
  transform: translateY(-2px);
  box-shadow: 0 10px 28px rgba(23, 25, 31, 0.1);
}

.title {
  margin: 0 0 8px;
  font-size: 18px;
  color: #17191f;
}
.price {
  font-size: 16px;
  color: #0d766e;
  font-weight: 700;
}

.category {
  display: inline-block;
  background: #eceee8;
  border-radius: 999px;
  padding: 4px 10px;
  font-size: 12px;
  color: #52605a;
  margin-top: 8px;
}

.badge {
  display: inline-block;
  background: #0d766e;
  color: #fff;
  border-radius: 999px;
  padding: 2px 8px;
  font-size: 11px;
  font-weight: 700;
}
```

```tsx
// app/products/page.tsx
import styles from '../styles/products.module.css'

const products = [
  { id: 1, name: 'เสื้อ', price: 299, category: 'แฟชั่น', badge: 'ใหม่' },
  { id: 2, name: 'กระเป๋า', price: 599, category: 'แฟชั่น', badge: 'ลดราคา' },
  { id: 3, name: 'แก้ว', price: 99, category: 'ของใช้', badge: 'ขายดี' },
]

export default function ProductsPage() {
  return (
    <div className={styles.grid}>
      {products.map((p) => (
        <div key={p.id} className={styles.card}>
          <span className={styles.badge}>{p.badge}</span>
          <h3 className={styles.title}>{p.name}</h3>
          <p className={styles.price}>{p.price} บาท</p>
          <span className={styles.category}>{p.category}</span>
        </div>
      ))}
    </div>
  )
}
```

### Layout

```tsx
// app/layout.tsx
import './styles/variables.scss'
import './styles/layout.scss'
import './styles/global.scss'
import '@fontsource/sarabun/400.css'
import '@fontsource/sarabun/700.css'

export const meta = {
  title: 'ร้านค้า',
  description: 'ร้านค้าออนไลน์',
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="th">
      <body>{children}</body>
    </html>
  )
}
```

---

## Under the Hood: Reproducible Class Names

CSS Module class names สามารถ reproducible ข้ามเครื่องและข้าม build:

```
pattern: {stem}_{local}__{hash:016x}

input:  /project/styles/card.module.css, class ".card"
hash:   fnv1a_64("styles/card.module.css:card")
output: card_card__feff5ad3a1e67b7b

input:  /project/app/components/Button.module.css, class ".primary"
hash:   fnv1a_64("app/components/button.module.css:primary")
output: Button_primary_1a2b3c4d5e6f7890
```

**สำคัญ:** hash คำนวณจาก **project-relative path** (lowercase, normalized) + `:` + class name —
ดังนั้น class name จะเหมือนเดิมทุก build ตราบใดที่ path และ class name ไม่เปลี่ยน

`normalized_relative_path` functions:

```rust
fn normalized_relative_path(path, project_root) -> String {
    // strip project_root → relative path
    // replace \ → / (cross-platform)
    // to_ascii_lowercase()
}
```

---

## Choosing an Approach

| วิธีการ                   | เหมาะสำหรับ                                          | ไม่เหมาะสำหรับ                                         |
| ------------------------- | ---------------------------------------------------- | ------------------------------------------------------ |
| Tailwind CSS              | แอปเน้นความเร็วในการสร้าง, ดีไซน์สไตล์ Utility-first | คนที่ไม่ชอบ HTML Class รกๆ                             |
| CSS Modules               | แอปที่มี Component ซับซ้อนและใช้ซ้ำได้               | การแชร์สไตล์ที่ต้องใช้ซ้ำข้าม Component บ่อยๆ (Global) |
| Global CSS                | โปรเจกต์ขนาดเล็กมาก, สไตล์พื้นฐาน (Reset/Base)       | โปรเจกต์ขนาดใหญ่ที่ต้องการ Encapsulation               |
| CSS-in-JS (Style Objects) | ผู้พัฒนาที่ชอบเขียน Logic ร่วมกับสไตล์               | ระบบที่ต้องการ Framework-agnostic CSS                  |

---

## Responsive Design Patterns

ตัวอย่างการทำ Responsive แบบไม่ต้องพึ่งพา Tailwind:

```css
/* app/components/Card.module.css */
.card {
  display: grid;
  grid-template-columns: 1fr;
  gap: 16px;
}

/* รองรับจอขนาด 768px ขึ้นไป */
@media (min-width: 768px) {
  .card {
    grid-template-columns: repeat(2, 1fr);
  }
}

/* รองรับจอขนาด 1024px ขึ้นไป */
@media (min-width: 1024px) {
  .card {
    grid-template-columns: repeat(3, 1fr);
  }
}
```

---

## Theming with CSS Custom Properties

เราสนับสนุนการใช้ CSS Variables ในการทำ Theming (โดยเฉพาะ Dark/Light mode):

```css
/* app/global.css */
:root {
  --bg-color: #ffffff;
  --text-color: #111827;
  --primary-color: #3b82f6;
}

[data-theme='dark'] {
  --bg-color: #111827;
  --text-color: #f9fafb;
  --primary-color: #60a5fa;
}

body {
  background-color: var(--bg-color);
  color: var(--text-color);
}
```

---

## Critical CSS Extraction

ในกระบวนการ Build, Ruvyxa จะดึง CSS ส่วนสำคัญ (Critical CSS) ที่จำเป็นต่อการแสดงผล "เหนือขอบจอ
(Above the fold)" ของทุกเพจ และฝัง (Inline) เข้าไปใน HTML ของหน้าเพจนั้น
เพื่อให้เบราว์เซอร์เรนเดอร์ได้ทันทีโดยไม่ต้องรอโหลดไฟล์ `.css`

---

## CSS Animation Performance

พยายามแอนิเมทเฉพาะคุณสมบัติที่เบราว์เซอร์สามารถประมวลผลผ่าน GPU ได้ (เช่น `transform`, `opacity`)
เพื่อประสิทธิภาพสูงสุด:

```css
/* ❌ ไม่ดี: ทำให้เกิด Layout Reflow */
.box {
  transition: margin-left 0.3s ease;
}
.box:hover {
  margin-left: 20px;
}

/* ✅ ดี: ใช้ GPU Hardware Acceleration */
.box {
  transition: transform 0.3s ease;
}
.box:hover {
  transform: translateX(20px);
}
```

---

## Print Stylesheets

คุณสามารถกำหนดรูปแบบสำหรับเวลาพิมพ์ (Print) โดยใช้ Media Query แบบ `print`:

```css
@media print {
  header,
  footer,
  nav,
  .hide-on-print {
    display: none !important;
  }

  body {
    background: white;
    color: black;
    font-size: 12pt;
  }
}
```

---

## Font Loading

เพื่อประสิทธิภาพสูงสุด ควรโหลดฟอนต์ผ่านคำสั่ง `@font-face` แทนการฝังผ่าน External CSS จาก Google
Fonts โดยตรง:

```css
@font-face {
  font-family: 'Inter';
  font-style: normal;
  font-weight: 400;
  font-display: swap; /* ทำให้ข้อความแสดงขึ้นมาก่อนฟอนต์จะโหลดเสร็จ */
  src: url('/fonts/Inter-Regular.woff2') format('woff2');
}
```

---

## Dark Mode with prefers-color-scheme

คุณสามารถจัดการโหมดมืดอัตโนมัติตามการตั้งค่าระบบของผู้ใช้งาน:

```css
:root {
  --background: white;
  --text: black;
}

@media (prefers-color-scheme: dark) {
  :root {
    --background: black;
    --text: white;
  }
}
```

---

## CSS Utility Classes

นอกเหนือจาก Tailwind แล้ว คุณสามารถสร้างชุด Utility เล็กๆ ของคุณเองได้ถ้าโปรเจกต์ไม่ได้ใหญ่พอจะใช้
Tailwind:

```css
/* utilities.css */
.flex {
  display: flex;
}
.flex-col {
  flex-direction: column;
}
.items-center {
  align-items: center;
}
.justify-center {
  justify-content: center;
}
.mt-4 {
  margin-top: 1rem;
}
.text-center {
  text-align: center;
}
```

---

## PostCSS and Autoprefixer

Ruvyxa ฝัง Autoprefixer และ PostCSS มาให้แล้ว เบื้องหลังจะเพิ่ม Vendor Prefix ให้อัตโนมัติ:

```css
/* ก่อน Build */
.example {
  user-select: none;
}

/* หลัง Build */
.example {
  -webkit-user-select: none;
  user-select: none;
}
```

---

## CSS Ordering Deep Dive

หากมี CSS หลายสไตล์ซ้อนทับกัน (Specificity ชนกัน) โค้ดที่โหลดทีหลังจะชนะ:

1. `<link rel="stylesheet">` จาก `css.entries` โหลดก่อนเสมอ
2. สไตล์ของ Server Component โหลดตาม
3. สไตล์ของ Client Component (ที่โหลดตอน Hydration)

คุณสามารถควบคุมเรื่องนี้ได้โดยใช้ฟีเจอร์ใหม่ `@layer` ใน CSS:

```css
@layer base, components, utilities;

@layer base {
  h1 {
    font-size: 2rem;
  }
}

@layer utilities {
  .text-xl {
    font-size: 1.5rem;
  }
}
```

---

## Build Output

ในโหมด Build ไฟล์ CSS ทั้งหมดที่ไม่ได้มาจาก Client Modules จะถูกรวมและแบ่งออกเป็นไฟล์หลัก (Chunks)
ภายในโฟลเดอร์ `.ruvyxa/static/css/` ซึ่งมี Hash ในชื่อไฟล์ เพื่อประโยชน์เรื่อง Browser Caching
ระยะยาว:

```
.ruvyxa/static/css/
├── global.7f8a9b.css
├── app_page.2b3c4d.css
└── _about_page.1a2b3c.css
```

---

## ข้อผิดพลาดทั่วไป

| ปัญหา                            | สาเหตุ                          | วิธีแก้                                                |
| -------------------------------- | ------------------------------- | ------------------------------------------------------ |
| SCSS compile error               | syntax ผิด (ลืม `;`, `}`)       | ตรวจสอบไฟล์ .scss                                      |
| CSS Module import ไม่ได้         | ใช้ `.module.css` หรือเปล่า     | เฉพาะ `.module.css`/`.module.scss`/`.module.sass`      |
| style ไม่มาใน production         | import ไม่ถูก path              | เช็ค relative path                                     |
| class name ชนกัน                 | ใช้ global class แทน CSS Module | เปลี่ยนเป็น CSS Module                                 |
| font ไม่แสดง                     | import font แล้ว?               | import ใน layout                                       |
| HMR ไม่ทำงาน                     | แก้ไขไฟล์ที่ไม่ได้ import       | import ไฟล์นั้นก่อน                                    |
| `@use` error ใน SCSS             | ใช้ `@import` แทน               | เปลี่ยนเป็น `@use`                                     |
| import css จาก npm ไม่ได้        | path ผิด                        | ใช้ full path จาก package                              |
| `@import "tailwindcss"` ไม่ทำงาน | ไม่ได้ติดตั้ง package           | `pnpm add tailwindcss && pnpm add -D @tailwindcss/cli` |
| CSS entry ไม่ถูก compile         | path ผิด/config ผิด             | ใช้ path ที่ relative กับ project root                 |

### RUV Error Codes ที่เกี่ยวข้อง

| Code    | ความหมาย                                | คำอธิบาย                             |
| ------- | --------------------------------------- | ------------------------------------ |
| RUV1400 | Tailwind CSS compilation failed         | Tailwind CLI return error            |
| RUV1401 | Tailwind CSS CLI not found              | ไม่พบ `@tailwindcss/cli`             |
| RUV1402 | Sass compilation failed                 | grass compile error                  |
| RUV1403 | Stylesheet import could not be resolved | import path ผิดหรือไฟล์ไม่มี         |
| RUV1404 | CSS entry must stay inside project root | `css.entries` อ้างอิงไฟล์นอก project |
| RUV1004 | Page is missing a default export        | ไฟล์ page ไม่มี export default       |

### Common SCSS Mistakes

```scss
// ผิด: ใช้ @import (deprecated)
@import './variables';

// ถูก: ใช้ @use
@use './variables' as *;

// ผิด: ไม่มี semicolon
$color: red

// ถูก: มี semicolon
$color: red;

// ผิด: import partial ด้วย underscore
@use './_variables';

// ถูก: ไม่ต้องมี underscore
@use './variables';

// ผิด: import CSS จาก npm ด้วย relative path
@use '../../../node_modules/package/style';

// ถูก: ใช้ bare specifier
@use 'package/style';
```

---

## Best Practices

```
Layout Pattern:
app/layout.tsx
  ├── import './globals.css'         ← Global styles (reset, typography)
  ├── import './styles/variables'    ← SCSS variables
  └── import '@fontsource/...'       ← External fonts

Component Pattern (CSS Modules):
app/components/Button.tsx
  └── import styles from './Button.module.css'
      ← Scoped classes, ไม่ชนกัน

SCSS Pattern:
app/styles/
  ├── _variables.scss   ← Variables (partial, @use by others)
  ├── _mixins.scss      ← Mixins (partial, @use by others)
  └── layout.scss       ← layout styles (imported in layout.tsx)
```

### หลักการ

1. **CSS Modules สำหรับ component** — ป้องกัน class name ชนกัน
2. **Global CSS สำหรับ base styles** — reset, typography, CSS custom properties
3. **SCSS สำหรับ logic ที่ซับซ้อน** — loops, mixins, functions, variables
4. **css.entries สำหรับไฟล์นอก app/** — shared styles, npm packages, project-level CSS
5. **HMR ช่วยให้แก้ style ได้เร็ว** — แก้ SCSS → browser อัพเดตทันทีโดยไม่ reload
6. **ใช้ Tailwind เมื่อต้องการ utility-first** — ใช้ `@import "tailwindcss"` ใน entry CSS
7. **ระวัง `@import` (deprecated)** — ใช้ `@use` แทนเสมอ

---

## ลองทำดู

สร้างกล่องโต้ตอบการลบแบบมีสไตล์:

**1. `app/components/DeleteDialog.module.css`**

```css
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
}
.dialog {
  background: white;
  padding: 24px;
  border-radius: 8px;
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.1);
}
.dangerButton {
  background: red;
  color: white;
  padding: 8px 16px;
  border-radius: 4px;
}
```

**2. `app/components/DeleteDialog.tsx`**

```tsx
import styles from './DeleteDialog.module.css'

export function DeleteDialog({ onConfirm, onCancel }) {
  return (
    <div className={styles.overlay}>
      <div className={styles.dialog}>
        <h2>ยืนยันการลบ?</h2>
        <p>การกระทำนี้ไม่สามารถย้อนกลับได้</p>
        <div>
          <button onClick={onCancel}>ยกเลิก</button>
          <button className={styles.dangerButton} onClick={onConfirm}>
            ลบข้อมูล
          </button>
        </div>
      </div>
    </div>
  )
}
```

---

## วิธีที่ Framework รวบรวม Styles

Style collection เริ่มจาก application scripts แล้วเดินตาม import graph และรับ `css.entries` แบบ
project-relative จาก config ด้วย ซึ่งเหมาะกับ global stylesheet ที่ตั้งใจไม่ import จาก application
code หาก import stylesheet หาไม่พบจะรายงาน `RUV1403`; การเพิ่ม path ใน `css.entries` ควรทำเฉพาะเมื่อ
stylesheet นั้นเป็น global จริง ไม่ใช่ใช้ซ่อน relative import ที่พัง

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'

export default config({
  css: {
    entries: ['styles/print.css', 'styles/tokens.css'],
  },
})
```

paths เหล่านี้ถูก validate พร้อม config ส่วนอื่น เก็บ stylesheet ของ component ให้ import จาก
component นั้นเมื่อทำได้ เพื่อให้ dependency path อ่านง่ายทั้งตอน development และ production
collection

### CSS Modules กับ Sass มีคนละขั้น

ไฟล์ที่ลงท้าย `.module.css`, `.module.scss` หรือ `.module.sass` คือ CSS Modules สำหรับ Sass ระบบจะ
compile ก่อน แล้ว scope local class selectors ด้วย project-relative path และชื่อ class ที่คงที่ map
ที่ ได้คือสิ่งที่ TypeScript module import:

```tsx
import styles from './Button.module.scss'

export function Button({ children }: { children: React.ReactNode }) {
  return <button className={styles.primary}>{children}</button>
}
```

ใช้ `:global(...)` เฉพาะ selector ที่ต้อง escape module scoping จริง ๆ ไม่ใช่สิ่งแทน global
stylesheet; กฎกว้าง ๆ ควรอยู่ใน global CSS ที่ import หรือไฟล์ใน `css.entries`

### ตรวจอะไรเมื่อ Style ไม่ปรากฏ

1. ตรวจว่า stylesheet ถูก import จาก application module ที่เข้าถึงได้ หรือระบุใน `css.entries`
2. ตรวจ relative path และ extension ให้ตรง; import ที่ resolve ไม่ได้จะไม่ถูกข้ามแบบเงียบ ๆ
3. สำหรับ Sass ให้แก้ compiler error แทนคาดหวัง stylesheet ที่ compile ได้บางส่วน
4. รัน route/analysis ตามปกติหลังเปลี่ยน shared style entry

```bash
ruvyxa analyze --format human
ruvyxa trace /
npm run build
```

Development เสิร์ฟ CSS ที่รวบรวมไว้ขณะ watch ไฟล์ ส่วน production rendering จะ minify CSS
ชุดนั้นก่อน ฝังใน document จึงควรทดสอบ production build ด้วยเมื่อเกี่ยวข้องกับ order หรือกฎที่ไวต่อ
minification

---

## ขั้นตอนถัดไป

กลับไปที่ **[ดัชนี](../../index.md)** หรือดูบทอื่นๆ:

- **[01-getting-started.md](./01-getting-started.md)** — เริ่มต้นใช้งาน
- **[02-routing.md](./02-routing.md)** — Routing
- **[03-server-client-components.md](./03-server-client-components.md)** — Server & Client
  Components
- **[04-rendering-strategies.md](./04-rendering-strategies.md)** — กลยุทธ์การ Render
- **[05-data-loading-cache.md](./05-data-loading-cache.md)** — การโหลดข้อมูลและ Cache
- **[06-server-actions.md](./06-server-actions.md)** — Server Actions
- **[07-api-routes.md](./07-api-routes.md)** — API Routes
