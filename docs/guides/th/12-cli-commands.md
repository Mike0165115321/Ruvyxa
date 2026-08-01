# คำสั่ง CLI

แพ็กเกจ `ruvyxa` ติดตั้ง Ruvyxa CLI โดยนิยามคำสั่งและ flag อยู่ที่ `crates/ruvyxa_cli/src/main.rs`
ดังนั้นผลจาก `--help` ของ binary ที่ติดตั้งคือข้อมูลอ้างอิงของ flag ที่ถูกต้องที่สุด

```bash
ruvyxa --help
```

> Flag เป็นของแต่ละคำสั่ง ไม่ใช่ global flag: ไม่มี `--verbose`, `--no-color`, `--open` หรือ flag
> สำหรับ bundle analysis ใน CLI ปัจจุบัน และ `--root`/`--runtime` ใช้ได้เฉพาะคำสั่งที่ระบุไว้
> ด้านล่าง

## ภาพรวมคำสั่ง

| คำสั่ง          | หน้าที่                                                              |
| --------------- | -------------------------------------------------------------------- |
| `dev`           | เปิด development server พร้อม route watching และ HMR                 |
| `build`         | สร้าง production build และเลือก deployment adapter ได้               |
| `check`         | รัน TypeScript check (เมื่อมี `tsconfig.json`) แล้วรัน `test:parity` |
| `start`         | เปิดใช้ production build ที่สร้างไว้แล้ว                             |
| `preview`       | เปิดใช้ production build เพื่อดูในเครื่อง                            |
| `routes`        | แสดง route table ที่ค้นพบ                                            |
| `analyze`       | ตรวจ routes, imports และ server/client boundary                      |
| `doctor`        | ตรวจ project setup, dependencies, runtime และ adapter                |
| `clean`         | ลบ generated build output ตาม config                                 |
| `trace`         | แสดง manifest entry ของ route เดียว                                  |
| `bench`         | วัด route discovery, analysis และ production build                   |
| `test:parity`   | เปรียบเทียบพฤติกรรม dev/production และ smoke-render page routes      |
| `plugin create` | สร้างโครง package สำหรับ plugin ที่ publish ได้                      |

`test:parity` ใช้ alias `parity` ได้ด้วย

## Options ที่ใช้ร่วมกัน

คำสั่งที่รับ project root มี `--root <PATH>` และใช้ directory ปัจจุบันเป็นค่าเริ่มต้น คำสั่ง `dev`,
`build`, `check`, `start`, `preview`, `routes`, `analyze`, `doctor`, `clean` และ `test:parity` รับ
`--runtime <node|bun>` ด้วย โดย flag นี้มีความสำคัญเหนือกว่า `RUVYXA_RUNTIME` และ `config.runtime`

ทุกคำสั่งมี `-h` / `--help`

## `ruvyxa dev`

```bash
ruvyxa dev [--root <PATH>] [--host <HOST>] [--port <PORT>] [--runtime <node|bun>]
```

ใช้สำหรับพัฒนาแอป โดย `host` และ `port` เป็นค่าเลือกได้; ค่าเริ่มต้นสุดท้ายมาจาก config ของ project
และ server

```bash
ruvyxa dev
ruvyxa dev --port 4000
ruvyxa dev --root ../other-app --runtime bun
```

## `ruvyxa build`

```bash
ruvyxa build [--root <PATH>] [--target <node|bun|edge|static>] \
  [--adapter <NAME_OR_NPM_PACKAGE>] [--runtime <node|bun>]
```

`--target` ใช้ override build target ส่วน `--adapter` เลือก adapter โดยไม่ต้องแก้ `ruvyxa.config.ts`
ชื่อที่มีในตัวคือ `node`, `bun`, `static`, `vercel`, `netlify`, `cloudflare`, `railway`, `render`,
`firebase` และ `aws` หรือใช้ชื่อ npm package ที่ถูกต้องได้

```bash
ruvyxa build
ruvyxa build --target static
ruvyxa build --adapter vercel
```

โฟลเดอร์ output กำหนดด้วย `outDir` (starter ใช้ `.ruvyxa`) ดูรายละเอียดขั้นตอน build ได้ที่
[CLI Architecture](../../architecture/cli.md) และ [Deployment](./13-deployment.md)

## `ruvyxa check`

```bash
ruvyxa check [--root <PATH>] [--runtime <node|bun>]
```

คำสั่งนี้รัน `tsc --noEmit` เมื่อพบ `tsconfig.json` แล้วรัน parity flow เดียวกับ
`ruvyxa test:parity`

```bash
npm run check
ruvyxa check --runtime bun
```

## `ruvyxa start` และ `ruvyxa preview`

```bash
ruvyxa start [--root <PATH>] [--host <HOST>] [--port <PORT>] [--runtime <node|bun>]
ruvyxa preview [--root <PATH>] [--host <HOST>] [--port <PORT>] [--runtime <node|bun>]
```

ทั้งสองคำสั่ง serve production build ที่มีอยู่แล้ว จึงต้อง `ruvyxa build` ก่อน `preview` เป็น
คำสั่งสำหรับ local-preview โดยเฉพาะและไม่ได้ build ให้อัตโนมัติ

```bash
npm run build
npm run start
ruvyxa preview --port 4173
```

## `ruvyxa routes`

```bash
ruvyxa routes [--root <PATH>] [--runtime <node|bun>]
```

แสดง routes ที่ค้นพบจาก application directory ตาม config ใช้ตรวจ route paths หรือหา conflict

```bash
ruvyxa routes
```

## `ruvyxa analyze`

```bash
ruvyxa analyze [--root <PATH>] [--runtime <node|bun>] \
  [--format <auto|human|json|sarif>] [--output <PATH>]
```

ตรวจ routes, imports และ server/client boundaries ค่า `auto` รักษาพฤติกรรมตาม terminal หรือ piped
output และ `--output` เขียนรายงานตาม format ที่เลือกลงไฟล์

```bash
ruvyxa analyze
ruvyxa analyze --format sarif --output reports/ruvyxa.sarif
```

## `ruvyxa doctor`

```bash
ruvyxa doctor [--root <PATH>] [--target <node|bun|edge|static>] \
  [--adapter <NAME_OR_NPM_PACKAGE>] [--runtime <node|bun>] [--json]
```

ใช้ `--adapter` เพื่อตรวจ adapter โดยไม่สร้าง artifacts และใช้ `--json` เพื่อรับ compatibility
report แบบ JSON

```bash
ruvyxa doctor
ruvyxa doctor --adapter cloudflare --json
```

## `ruvyxa clean`

```bash
ruvyxa clean [--root <PATH>] [--runtime <node|bun>]
```

ลบเฉพาะ generated build directory ตาม config ของ Ruvyxa ไม่ลบ dependencies หรือไฟล์ project อื่น

```bash
ruvyxa clean
```

## `ruvyxa trace`

```bash
ruvyxa trace <ROUTE> [--root <PATH>]
```

ต้องระบุ `<ROUTE>` คำสั่งจะสร้าง manifest ปัจจุบันและแสดง entry ที่ตรงกัน

```bash
ruvyxa trace /
ruvyxa trace /blog/[slug]
```

## `ruvyxa bench`

```bash
ruvyxa bench [--root <PATH>] [--samples <COUNT>] [--json]
```

ค่าเริ่มต้นของ `--samples` คือ `3`; ใช้ `--json` เมื่อให้ CI หรือเครื่องมืออื่นอ่านผล

```bash
ruvyxa bench --samples 5
ruvyxa bench --json
```

## `ruvyxa test:parity`

```bash
ruvyxa test:parity [--root <PATH>] [--runtime <node|bun>]
```

เปรียบเทียบ route manifests ของ dev/production และ smoke-render page routes เป็น parity check
เดียวกับที่ `ruvyxa check` เรียกใช้

```bash
ruvyxa test:parity
ruvyxa start
```

## `ruvyxa plugin create`

```bash
ruvyxa plugin create <NAME> [--root <PATH>] [--dir <PATH>]
```

ต้องระบุ `<NAME>` และ `--dir` เป็น path สัมพัทธ์กับ `--root`; หากไม่ระบุจะสร้างใน directory
ชื่อเดียวกับ plugin

```bash
ruvyxa plugin create my-plugin
ruvyxa plugin create @acme/analytics --dir packages/analytics
```

ดู TypeScript plugin contract ได้ที่ [Plugins](./14-plugins.md)

## Scripts ใน Minimal Starter

Minimal starter ปัจจุบันมี scripts ดังนี้

```json
{
  "dev": "ruvyxa dev",
  "build": "ruvyxa build",
  "start": "ruvyxa start",
  "typecheck": "tsc --noEmit",
  "check": "ruvyxa check"
}
```

คำสั่งที่ไม่มี script ให้เรียกผ่าน `ruvyxa` โดยตรง เช่น `ruvyxa analyze --format json`

## Recipes สำหรับใช้งานจริง

ตัวอย่างด้านล่างใช้คำสั่ง CLI โดยตรงสำหรับคำสั่งที่ minimal starter ยังไม่มี script ให้ เปลี่ยน
`../my-app` เป็น path ของโปรเจกต์ตามจริงได้

### เปิดโปรเจกต์บน Port อื่น

เมื่อมี service อื่นใช้ port เดิมอยู่ ให้ระบุ port ที่ต้องการ:

```bash
ruvyxa dev --port 4000
```

หากต้องการให้เครื่องอื่นในเครือข่ายเข้าถึงได้ ให้ระบุ host ด้วย:

```bash
ruvyxa dev --host 0.0.0.0 --port 4000
```

เปิด terminal นี้ค้างไว้ระหว่างพัฒนา เพราะ HMR และ route watching ทำงานอยู่ใน process เดียวกัน

### ทำงานกับ Project คนละ Directory

ใช้ `--root` เพื่อสั่ง CLI กับ project อื่นโดยไม่ต้อง `cd`:

```bash
ruvyxa dev --root ../my-app
ruvyxa routes --root ../my-app
ruvyxa analyze --root ../my-app --format human
```

`--root` มีผลต่อคำสั่งนั้นเท่านั้น ใน script จึงควรส่ง flag นี้ให้ทุกคำสั่งที่ต้องใช้

### ทดลองรันด้วย Bun

หาก project ตั้งค่า Node อยู่ แต่ต้องการทดสอบด้วย Bun ให้ใช้ runtime override ของคำสั่งนั้น:

```bash
ruvyxa dev --runtime bun
ruvyxa build --runtime bun
ruvyxa check --runtime bun
```

คำสั่งนี้ไม่แก้ `ruvyxa.config.ts` และไม่เปลี่ยน environment ถาวร

### Build และเปิด Local Production Server

ขั้นตอนปกติคือ build ก่อน แล้วจึง serve output ที่สร้างไว้:

```bash
npm run build
npm run start
```

หากต้องการระบุ target อย่างชัดเจน:

```bash
ruvyxa build --target node
ruvyxa start --port 3001
```

`start` และ `preview` อ่าน build ที่มีอยู่แล้ว จึงไม่ build ให้อัตโนมัติ

### Preview Static Build

ใช้ static target เมื่อ routes และ adapter ที่เลือก รองรับผลลัพธ์ของแอป:

```bash
ruvyxa build --target static --adapter static
ruvyxa preview --port 4173
```

หาก route strategy ไม่รองรับ platform adapter ระบบจะรายงาน incompatibility ระหว่าง build ดู
[Deployment](./13-deployment.md) สำหรับขั้นตอนของแต่ละ adapter

### เลือก Hosting Adapter ชั่วคราว

`--adapter` เหมาะกับ CI หรือการทดลอง target โดยไม่ต้องแก้ config:

```bash
ruvyxa build --adapter vercel
ruvyxa build --adapter cloudflare
ruvyxa build --adapter @acme/ruvyxa-adapter-node
```

ตัวอย่างสุดท้ายเป็นชื่อ package ซึ่งต้อง resolve ได้ใน environment ที่รัน build

### ตรวจความพร้อมก่อน Commit

คำสั่งที่สั้นและทำซ้ำได้สำหรับตรวจแอปคือ:

```bash
npm run check
```

เมื่อมี `tsconfig.json` คำสั่งจะรัน `tsc --noEmit` แล้วตามด้วย route/render parity flow หากต้องการ
รันเฉพาะ parity:

```bash
ruvyxa test:parity
# alias ที่เท่ากัน
ruvyxa start
```

### ตรวจ URL หรือ Route ที่มีปัญหา

เริ่มจากดู route table แล้วตรวจ route ที่สนใจ:

```bash
ruvyxa routes
ruvyxa trace /about
ruvyxa trace /blog/[slug]
```

`trace` รับ route pattern ไม่ใช่ชื่อไฟล์ สำหรับ dynamic page ให้ใช้ `/blog/[slug]` ไม่ใช่ URL จริง
เช่น `/blog/hello`

### สร้าง Analysis Report สำหรับ Tool อื่น

ใช้ JSON เมื่อโปรแกรมอื่นจะอ่านผล และใช้ SARIF เมื่อ code-scanning tool รองรับ:

```bash
ruvyxa analyze --format json --output reports/ruvyxa-analysis.json
ruvyxa analyze --format sarif --output reports/ruvyxa.sarif
```

สร้าง directory ปลายทางก่อนหากยังไม่มี และใช้ `--format human` เมื่ออยากอ่านใน terminal

### ตรวจ Toolchain และ Adapter

รัน `doctor` ก่อน deploy ครั้งแรก หรือหลังเปลี่ยน runtime/adapter:

```bash
ruvyxa doctor
ruvyxa doctor --target edge
ruvyxa doctor --adapter cloudflare --json
```

`--adapter` ตรวจ adapter contract โดยไม่สร้าง build artifacts

### ล้างเฉพาะ Generated Output

เมื่อต้องการสร้าง local build ใหม่ ให้ล้าง output ของ Ruvyxa แล้ว build อีกครั้ง:

```bash
ruvyxa clean
ruvyxa build
```

คำสั่งนี้จำกัดอยู่ที่ output directory ตาม config ไม่ลบ `node_modules`, source files หรือ cache
directory อื่นโดยพลการ

### วัดผลหลายรอบ

ระบุจำนวน sample ให้คงที่เมื่อเปรียบเทียบการเปลี่ยนแปลงสองชุด:

```bash
ruvyxa bench --samples 5
ruvyxa bench --samples 5 --json
```

ควรเปรียบเทียบในเครื่อง, dependency state และจำนวน sample เดียวกัน เพราะ benchmark เป็นสัญญาณ เฉพาะ
environment ที่รัน

### สร้าง Plugin ใน Monorepo

กำหนด directory ของ package ได้อย่างชัดเจน:

```bash
ruvyxa plugin create analytics --root . --dir packages/analytics
```

หากเป็น standalone project ใช้ค่าเริ่มต้นได้:

```bash
ruvyxa plugin create my-plugin
```

จากนั้นดู [Plugins](./14-plugins.md) เพื่อเขียนและ register package ที่สร้างขึ้น

### ตัวอย่าง CI

หลังติดตั้ง dependencies แล้ว สามารถเรียก CLI ใน CI โดยตรง:

```bash
npm run check
ruvyxa analyze --format sarif --output reports/ruvyxa.sarif
ruvyxa build --adapter node
```

อย่าเพิ่ม flag ที่ CLI ปัจจุบันไม่มี เช่น `--verbose`, `--no-cache` หรือ `--sourcemap`

## Workflows แบบมีคำอธิบาย

ตัวอย่างต่อไปนี้แสดงลำดับที่คำสั่งทำงานร่วมกัน โดยจงใจแสดงเฉพาะคำสั่งที่มีอยู่จริงในปัจจุบัน ไม่เดา
output ขึ้นมาเอง เพราะ routes, config และ adapter ที่ติดตั้งแตกต่างกันในแต่ละ project

### เริ่มพัฒนาโปรเจกต์ครั้งแรก

เริ่มจากตรวจว่าระบบค้นพบ routes อะไรบ้าง แล้วจึงเปิด development server ค้างไว้ระหว่างแก้โค้ด:

```bash
# รันจาก application root
ruvyxa routes
ruvyxa dev
```

หากแอปอยู่ใน directory ข้างเคียง ใช้ลำดับเดิมโดยส่ง `--root` ให้ทั้งสองคำสั่ง:

```bash
ruvyxa routes --root ../my-app
ruvyxa dev --root ../my-app
```

ใช้ `routes` ก่อน `dev` เมื่อไม่แน่ใจว่า page ถูกค้นพบหรือไม่ เพราะ `routes` แค่รายงาน route table
และไม่เปิด server ส่วน `dev` ต้องเปิดค้างไว้ขณะเขียนโค้ด เนื่องจากเป็น process ที่ดูแล watcher และ
HMR

### ตรวจ Route ที่ทำงานไม่ตรงคาด

เมื่อมีปัญหากับ route ให้เริ่มจากการค้นหาแบบกว้าง ไปยัง manifest entry ของ route นั้น แล้วตามด้วย
static checks:

```bash
ruvyxa routes
ruvyxa trace /blog/[slug]
ruvyxa analyze --format human
ruvyxa check
```

แทน `/blog/[slug]` ด้วย pattern ที่ `routes` แสดง `trace` รับ route pattern นี้ ไม่รับชื่อ component
file หรือ URL ที่ใส่ parameter จริง `analyze` ตรวจโครงสร้างและ boundary ของ project ส่วน `check` จะ
ตรวจ TypeScript เพิ่มเมื่อใช้ได้ และรัน parity flow คำสั่งเหล่านี้ไม่แก้ source file

### ตรวจ Adapter ก่อน Build

เมื่อจะย้ายแอปไปยัง platform ใหม่ ให้ตรวจ target และ adapter ที่เลือกก่อน:

```bash
ruvyxa doctor --target edge
ruvyxa doctor --target edge --adapter cloudflare --json
ruvyxa build --target edge --adapter cloudflare
```

สองคำสั่งแรกเป็น diagnostics คำสั่งสุดท้ายจึงเป็นขั้นที่ build และเรียก adapter ที่เลือก หาก project
จะทำงานบน Node server ให้ใช้ `--target node` และ `--adapter node`; adapter ที่เลือกควรสอดคล้องกับ
environment สำหรับ deploy

### สร้างไฟล์สำหรับ Code-quality Job

`analyze` เขียน artifact ให้ CI ได้โดยไม่ผูกกับผู้ให้บริการรายใด สร้าง directory ของรายงานก่อน แล้ว
เลือกรูปแบบที่ tool ถัดไปอ่านได้:

```bash
mkdir -p reports
ruvyxa analyze --format json --output reports/ruvyxa-analysis.json
ruvyxa analyze --format sarif --output reports/ruvyxa.sarif
```

บน PowerShell ให้สร้าง directory ด้วย `New-Item -ItemType Directory -Force reports` แทน
`mkdir -p reports` ไฟล์ JSON และ SARIF เป็นคนละ format จึงเลือกเพียงรูปแบบเดียวได้ หาก job ไม่ได้
ต้องใช้ทั้งคู่ และควรเก็บ `npm run check` เป็น gate แยกสำหรับ type-check และ parity flow

### Build ใหม่หลังเปลี่ยน Configuration

หลังเปลี่ยน config หรือ adapter และต้องการ generated output ชุดใหม่ ให้ใช้ลำดับนี้:

```bash
ruvyxa doctor --adapter vercel
ruvyxa clean
ruvyxa build --adapter vercel
ruvyxa preview
```

`clean` ลบเฉพาะ output directory ที่ Ruvyxa กำหนด ไม่ใช่การ reset dependencies และไม่แทนคำสั่งของ
package manager เช่น `npm install` วาง `preview` เป็นขั้นสุดท้ายเพราะมัน serve สิ่งที่ `build`
สร้างแล้ว

### เปรียบเทียบ Performance อย่างยุติธรรม

วัดก่อนและหลังการเปลี่ยนแปลงที่เจาะจง โดยใช้จำนวน sample และรูปแบบ output เดียวกัน:

```bash
ruvyxa bench --samples 10 --json
# แก้เฉพาะจุดหนึ่ง แล้วรันคำสั่งเดิมอีกครั้ง
ruvyxa bench --samples 10 --json
```

เก็บ JSON สองชุดใน CI หรือเทียบในเครื่อง อย่าสรุปว่าเป็น regression โดยตรงจากผลที่ใช้ runtime,
dependency tree หรือจำนวน sample คนละชุด

### ดู Help แทนการเดา Flag

CLI เป็นแหล่งที่ปลอดภัยที่สุดสำหรับตรวจ syntax ล่าสุดของคำสั่ง:

```bash
ruvyxa --help
ruvyxa build --help
ruvyxa analyze --help
ruvyxa plugin create --help
```

เปิดดู help ของคำสั่งนั้นก่อนใส่ลง script โดยเฉพาะ `--format` เป็นของ `analyze`, `--json` เป็นของ
`doctor` และ `bench`, และ `--dir` เป็นของ `plugin create`

## ขั้นตอนถัดไป

- [Configuration](./11-configuration.md) — กำหนดค่า input ของ CLI
- [Deployment](./13-deployment.md) — เลือกและกำหนดค่า adapter
- [Plugins](./14-plugins.md) — เขียนหรือ scaffold plugin

## Under the Hood: CLI Diagnostics

CLI commands แสดง diagnostics จาก route graph และ bundler ตัวอย่างเช่น `ruvyxa analyze` สามารถรายงาน
รหัส boundary ที่ยืนยันจาก source ได้แก่ `RUV1007` (`server-only` เข้า client graph), `RUV1008`
(private environment variable ใน client graph), `RUV1009` (`client-only` ถูกเรียกจาก server graph)
และ `RUV1010` (โมดูลใน `server/` เข้า client graph) โดยรหัสที่ได้ขึ้นอยู่กับกฎที่ละเมิด ไม่ควรถือว่า
ส่วน `ruvyxa doctor` ตรวจ adapter ที่เลือกและ capability ที่ adapter รายงาน

### Full CLI Capabilities

- `ruvyxa dev`: Boots the Axum server with HMR and persistent JS workers.
- `ruvyxa build`: Triggers Oxc compilation, Tree-shaking, and CSS fnv1a_64 hashing.
- `ruvyxa test:parity`: Evaluates behavioral drift between development and production routes.
