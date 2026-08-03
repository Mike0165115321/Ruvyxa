# คู่มือนักพัฒนา Ruvyxa

คู่มือนี้สำหรับผู้ร่วมพัฒนา framework ไม่ว่าจะเป็นผู้ที่แก้ Rust workspace, Ruvyxa CLI, npm package,
adapter, template, runtime หรือ integration fixture หากคุณกำลังสร้างแอปด้วย Ruvyxa ให้เริ่มที่
[คู่มือผู้ใช้](guides/index.md)

## 1. ความต้องการและการตั้งค่าในเครื่อง

ติดตั้ง Node.js 22.12 ขึ้นไป, pnpm 11 ขึ้นไป และ Rust 1.96 ขึ้นไป (Rust edition 2024) เวอร์ชัน Node
ขั้นต่ำสอดคล้องกับ native Oxc transformer ที่ runtime compiler ใช้
จากนั้นรันสคริปต์ตั้งค่าตามระบบปฏิบัติการที่ root ของ repository:

```powershell
.\setup.bat
```

บน macOS หรือ Linux ให้ใช้:

```bash
./setup.sh
```

สคริปต์จะติดตั้ง dependency ตาม lockfile, build ทุก package ใน workspace และ compile Ruvyxa CLI
แล้วตรวจ integration fixture ต่อด้วยคำสั่ง:

```bash
cargo run -p ruvyxa_cli -- doctor --root examples/demo
cargo run -p ruvyxa_cli -- routes --root examples/demo
```

ห้าม commit output ที่สร้างขึ้น ได้แก่ `target/`, `node_modules/`, `.ruvyxa/`, `dist/`, `.npm-pack/`
และ `.npm-smoke/` ทุกชั้นของระบบต้องแยก `RUVYXA_PUBLIC_` ที่ browser ใช้ได้ออกจาก secret
ที่ใช้ได้เฉพาะฝั่ง server เสมอ

## 2. แผนที่ repository

```text
npm package: ruvyxa
  └─ bin/ruvyxa.js -> Ruvyxa CLI binary สำหรับแต่ละแพลตฟอร์ม
       ├─ crates/ruvyxa_cli          คำสั่ง, การโหลด config และ orchestration ของ build
       ├─ crates/ruvyxa_graph        ค้นหา route, ตรวจ render mode และ validation
       ├─ crates/ruvyxa_bundler      compile TS/JSX/MDX, Oxc transform, resolve, link, map และ minify
       ├─ crates/ruvyxa_dev_server   Axum server, HMR, router, cache, Node/Bun worker pool และ CSS minification
       ├─ crates/ruvyxa_middleware   Tower middleware และ plugin bridge
       └─ crates/ruvyxa_diagnostics  diagnostic แบบมีโครงสร้าง RUV####

packages/
  ├─ ruvyxa                    CLI launcher, runtime bridge และ public re-export
  ├─ @ruvyxa/core              config และ server API, type และ adapter contract
  ├─ @ruvyxa/react             Image, Seo, hydration, loader และ error boundary
  ├─ @ruvyxa/auth              session, OAuth, magic-link, WebAuthn
  ├─ @ruvyxa/database          typed CRUD พร้อม adapter Prisma/DynamoDB/custom
  ├─ @ruvyxa/realtime          WebSocket transport ที่ขับเคลื่อนด้วย action
  ├─ @ruvyxa/adapter-*         package deployment adapter
  ├─ @ruvyxa/cli-*             native binary สำหรับแต่ละแพลตฟอร์ม (darwin-arm64, linux-arm64, linux-x64, win32-arm64, win32-x64)
  └─ create-ruvyxa             คำสั่ง scaffold และ packaging ของ minimal template
```

สัญญาของ framework มักคร่อมทั้ง Rust และ TypeScript การเปลี่ยน config, runtime file, package export
หรือพฤติกรรม starter จึงต้องตรวจทั้งสองฝั่ง อย่าเปลี่ยน TypeScript type แล้วสันนิษฐานว่า Ruvyxa CLI
จะยอมรับ เพราะ `ruvyxa_cli` deserialize runtime configuration แบบเข้มงวดแยกต่างหาก

## 3. วงจรการทำงานและการตรวจสอบ

ก่อนแก้ไข ให้อ่านโมดูลที่แตะ, caller โดยตรง, test และตัวอย่างใน demo ที่ใกล้ที่สุด เริ่มจาก check
ที่แคบที่สุด แล้วค่อยขยายเมื่อพฤติกรรมที่เปลี่ยนถูกใช้งานร่วมกัน

```bash
# งาน Rust เฉพาะจุด
cargo test -p ruvyxa_graph --locked
cargo test -p ruvyxa_cli --locked

# งาน JavaScript package เฉพาะจุด
pnpm --filter ruvyxa test
pnpm --filter ruvyxa check

# สัญญาณแบบ end-to-end ของแอป
cargo run -p ruvyxa_cli -- analyze --root examples/demo
cargo run -p ruvyxa_cli -- check --root examples/demo
cargo run -p ruvyxa_cli -- test:parity --root examples/demo
```

ก่อนส่งมอบงานที่กระทบ framework, runtime, template หรือ packaging
ให้รันชุดตรวจสอบแบบกว้างเท่าที่เกี่ยวข้อง:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --locked -- -D warnings
pnpm -r build
pnpm -r check
pnpm -r test
pnpm format:check
pnpm release:validate
pnpm pack:smoke
```

ถ้า Windows แจ้งว่า `target/debug/ruvyxa.exe` ถูกล็อก ให้หยุด development server หรือ process
อื่นที่กำลังใช้งาน executable นั้นก่อนลองใหม่ อย่าลบ `target/` ทั้งไดเรกทอรีเพื่อกลบปัญหา file lock

## 4. แผนที่การเปลี่ยนแปลง

| สิ่งที่เปลี่ยน                                               | พื้นที่หลัก                                   | หลักฐานขั้นต่ำ                          |
| ------------------------------------------------------------ | --------------------------------------------- | --------------------------------------- |
| คำสั่ง CLI, การ parse config, orchestration ของ build        | `crates/ruvyxa_cli/src/main.rs`               | Rust test ที่เกี่ยวข้องและ demo `check` |
| route matching, validation, การตรวจ render mode              | `crates/ruvyxa_graph/src/lib.rs`              | graph test และ `routes`/`analyze`       |
| compilation, linking, source map, Oxc transform/minification | `crates/ruvyxa_bundler`                       | bundler test และ demo build             |
| CSS collection, minification, style HMR                      | `crates/ruvyxa_dev_server/src/style.rs`       | crate test และ demo build               |
| พฤติกรรม API/action/HMR/server                               | `crates/ruvyxa_dev_server`                    | crate test และ parity                   |
| core config หรือ server API                                  | `packages/@ruvyxa/core/src`                   | package test/check                      |
| npm launcher หรือ runtime script                             | `packages/ruvyxa`                             | package test และ `pnpm pack:smoke`      |
| starter ที่สร้างจาก template                                 | `templates/minimal`, `packages/create-ruvyxa` | create-package test และ pack smoke      |
| พฤติกรรมแอปที่ตัดข้ามหลายส่วน                                | `examples/demo`                               | `analyze`, `check` และ `test:parity`    |

เพิ่ม Rust test ไว้ข้างพฤติกรรม Rust ที่ใช้ร่วมกัน เพิ่ม Node test ใต้ `tests/packages/**`
เมื่อเปลี่ยน public config, runtime, package หรือ template contract ห้ามลดความเข้มงวดของ test
เดิมเพียงเพื่อให้การเปลี่ยนผ่าน

## 5. Public contract ที่ต้องสอดคล้องกัน

### CLI

ชุดคำสั่งที่รองรับคือ:

```text
dev, build, check, start, preview, routes, analyze, doctor,
clean, trace, bench, test:parity (พร้อม alias parity)
```

คงชื่อคำสั่ง, ชื่อ option, ความหมายของ output และค่าเริ่มต้นของ build/root ที่เปิดเผยต่อผู้ใช้
เว้นแต่การเปลี่ยนแปลงนั้นตั้งใจเป็น breaking release

### Configuration

`ruvyxa.config.ts` คือสัญญาแบบเข้มงวด core package กำหนด TypeScript type ขณะที่ Ruvyxa CLI ตรวจและ
deserialize runtime representation เมื่อเพิ่ม field:

1. เพิ่ม type และเอกสารใน `packages/@ruvyxa/core`
2. เพิ่ม Rust config field ที่จับคู่ camelCase ถูกต้อง
3. ตรวจค่าที่ไม่ปลอดภัยหรือเป็นไปไม่ได้ใน Rust
4. เชื่อมค่าไปยัง development และ production server/build path
5. เพิ่ม test สำหรับค่าที่ยอมรับและค่าที่ปฏิเสธ
6. อัปเดตคู่มือผู้ใช้หากผู้สร้างแอปใช้งาน option นี้ได้

Key ของ configuration ที่ไม่รู้จักต้อง fail โดยตั้งใจ แทนที่จะถูกข้าม เพื่อป้องกัน typo
ที่เปลี่ยนพฤติกรรม deployment แบบเงียบ ๆ

### Routes, rendering และ boundary

- ปฏิเสธ route ที่ซ้ำหรือกำกวม แทนการตั้งลำดับความสำคัญที่ไม่ได้บันทึกไว้
- คงลำดับการตรวจ rendering: client directive, PPR, ISR, `getStaticParams`, static candidate แล้วจึง
  SSR
- คง validation ของ server/client สำหรับ `server-only`, `client-only`, import จาก `server/`
  และการเข้าถึง private environment
- เก็บ private variable ไว้ฝั่ง server; มีเพียงค่า `RUVYXA_PUBLIC_` ที่เข้า client bundle ได้

### Packaging

tarball ที่ publish ต้องไม่มี test และ dependency แบบ `workspace:` และต้องมี runtime script,
template file, platform binary และ launcher ทุกตัวที่คำสั่งสาธารณะต้องใช้

### Releases

`pnpm release:bump <version>` จะทำให้ workspace package, Rust crate และ starter dependency ใช้
release version เดียวกัน จากนั้น workflow release publish native CLI package ก่อน แล้วจึง shared
JavaScript package, ทุก package `@ruvyxa/adapter-*` ทางการ, `ruvyxa` และ `create-ruvyxa` ตามลำดับ
รายการและการตรวจ `npm install ruvyxa@<version>` ในสภาพแวดล้อมสะอาดอยู่ใน
`.github/workflows/release.yml` โดยตรง ดังนั้นเมื่อเพิ่ม adapter ต้องอัปเดต workflow นี้ใน change
เดียวกัน

## 6. Diagnostics

diagnostic ที่ผู้ใช้เห็นใช้รูปแบบ `RUV####` diagnostic ใหม่ควรมี:

1. รหัสในช่วงที่เหมาะสม
2. ชื่อที่กระชับ
3. คำอธิบายสัญญาที่ถูกละเมิด
4. ตำแหน่งไฟล์เมื่อทราบ
5. วิธีแก้ที่ทำตามได้จริง

อย่าแสดง build error ทั่วไปเมื่อ framework ระบุ source route, import, configuration field หรือ
boundary ที่ผิดได้ ให้เพิ่ม test สำหรับ diagnostic ใหม่
และอัปเดตคู่มือภาษาอังกฤษที่เกี่ยวข้องหากผู้ใช้ต้องดำเนินการตามมัน

## 7. Templates และ scaffold packaging

source starter คือ `templates/minimal/`, `templates/blog/`, `templates/crud/` และ
`templates/api-backend/` โดย `packages/create-ruvyxa/scripts/prepare-template.mjs`
จะคัดลอกทั้งสี่เข้า template ที่ถูก ignore ของ package ก่อน pack ให้พฤติกรรม starter
ที่ผู้ใช้สังเกตได้, รายการ template ของ CLI และ package test สอดคล้องกันเสมอ

npm จะไม่ใส่ `.gitignore` ที่ซ้อนอยู่ใน package tarball สคริปต์ prepare จึงเปลี่ยนชื่อไฟล์ใน package
เป็น `gitignore` แล้ว scaffold จึงคืนชื่อเป็น `.gitignore` ในแอปที่สร้างใหม่
พฤติกรรมนี้ตั้งใจไว้และครอบคลุมด้วย `pnpm pack:smoke` อย่าแทนด้วย npm ignore rule ที่ทำให้ไฟล์
ignore ของ starter หายไปอีก

starter ใช้ npm binary ปกติ:

```json
"build": "ruvyxa build"
```

เมื่อเปลี่ยน starter ให้ `dev`, `build`, `start` และ `check` ยังคงรูปแบบมาตรฐานเดียวกัน package
ไม่ใช่ทุกแอปที่นำไปใช้ เป็นผู้รับผิดชอบการ publish launcher พร้อม executable permission

## 8. ปัญหา executable bit บน Vercel

`ruvyxa` ประกาศ npm binary ที่ `packages/ruvyxa/bin/ruvyxa.js` ไฟล์นี้ต้อง executable (`100755`)
ทั้งใน Git และใน tarball ที่ publish ไม่เช่นนั้น environment เช่น Vercel อาจล้มเหลวก่อน framework
เริ่มทำงาน:

```text
node_modules/.bin/ruvyxa: Permission denied
```

ตรวจทั้งสองชั้น:

```bash
git ls-files --stage packages/ruvyxa/bin/ruvyxa.js
pnpm pack:smoke
```

Git mode ต้องขึ้นต้นด้วย `100755` ส่วน pack smoke จะตรวจ tar header, รัน Ruvyxa launcher
ที่แตกออกมาผ่าน Node, ตรวจ packed create command และยืนยันว่าแอปที่สร้างมี `.gitignore`

เมื่อเปลี่ยน launcher, การค้นหา Ruvyxa CLI binary, optional platform package หรือ package `files`
list ให้รัน `pnpm release:validate` และ `pnpm pack:smoke` เสมอ อย่าเชื่อเฉพาะ workspace symlink
เพราะเนื้อหาและ permission ของ package ที่ publish คือ deployment contract

## 9. Demo คือระบบ integration

`examples/demo` ไม่ได้เป็นเพียงตัวอย่าง แต่ทดสอบพฤติกรรม static, dynamic, catch-all, API, action,
MDX, environment, style และ rendering strategy ผ่านเส้นทางเดียวกับที่ผู้ใช้รัน ใช้ demo
เพื่อแยกปัญหาเรื่องความยืดหยุ่นและ parity:

```bash
pnpm --dir examples/demo doctor
pnpm --dir examples/demo routes
pnpm --dir examples/demo analyze
pnpm --dir examples/demo typecheck
pnpm --dir examples/demo check
```

ใช้ `analyze` ก่อนสำหรับปัญหา route/import/boundary ใช้ `check` สำหรับ type check, production build,
เปรียบเทียบ route ระหว่าง development/production และ page smoke render ใช้ `trace` เพื่อตรวจ
manifest entry เดียว ห้าม hard-code framework version หรือจำนวน route ใน demo health endpoint;
`doctor` และ `routes` คือแหล่งข้อมูล runtime ที่เชื่อถือได้

## 10. Boundary ที่ทราบและเอกสารอย่างตรงไปตรงมา

บันทึกเฉพาะสิ่งที่ source code และ test รองรับเท่านั้น

- การเลือก rendering strategy ใช้ source scanning ตามลำดับความสำคัญที่บันทึกไว้; สำหรับพฤติกรรม
  deployment สำคัญ ควรแนะนำ explicit route export
- path ของ configuration จำกัดอยู่ใน project root เพื่อป้องกัน traversal ส่วน style หรือ asset
  ภายนอกต้องมีวิธี import/copy ที่อยู่ในโปรเจกต์
- adapter package ส่งคืน output metadata ที่มี type และ build artifact แบบ declarative config
  renderer เรียก `adapter.build()` และ CLI เรียกซ้ำหลัง build เพื่อสร้าง artifact ใน staging ก่อน
  commit `build.json` Function artifact compile route TS/TSX เป็น static registry bundle
  ที่รันได้แบบ `.mjs`; handler โหลด registry นี้แทน raw manifest path ส่วน static adapter
  ตั้งใจไม่สร้าง serverless หรือ edge request handler
- `check` เป็นสัญญาณความพร้อมของแอป ไม่ใช่ browser E2E suite, load test หรือ security audit
  ให้เพิ่มการตรวจในชั้นที่ feature เปลี่ยนจริง

เอกสารที่ดูแลแบ่งให้ชัดเจน: `docs/guides/` สำหรับผู้สร้างแอป และ `docs/developer-guide.md`
สำหรับผู้ร่วมพัฒนา framework เมื่อ user journey เปลี่ยน ให้ root README, create-package README, demo
README, คำสั่ง, ค่าเริ่มต้น, ข้อจำกัดด้านความปลอดภัย และข้อความ deployment สอดคล้องกันเสมอ
