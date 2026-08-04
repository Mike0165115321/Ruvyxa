# CLI และ application script

[README](../../README.md) ที่ root คือภาพรวมโครงการที่ใช้อ้างอิงหลัก ภายใน Ruvyxa application ที่
สร้างแล้ว ให้ใช้ npm script ตามตารางด้านล่าง นี่คือ interface ที่ starter ทุกตัวเตรียมไว้และ
copy-paste ได้จริง โดยเฉพาะให้ใช้ `routes:json` และ `analyze:html` แทนการให้ผู้อ่านประกอบ flag หลัง
script ขึ้นเอง

| คำสั่งใน application                                                                                                                  | สิ่งที่รัน                              | วัตถุประสงค์                                                             |
| ------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------- | ------------------------------------------------------------------------ |
| `npm run dev`                                                                                                                         | `ruvyxa dev`                            | route watching และ hot reload                                            |
| `npm run build`                                                                                                                       | `ruvyxa build`                          | สร้าง production output                                                  |
| `npm run check`                                                                                                                       | `ruvyxa check`                          | ตรวจความพร้อมของ application                                             |
| `npm run start` / `npm run preview`                                                                                                   | `ruvyxa start` / `preview`              | serve หรือ preview local ของ build ที่มีอยู่                             |
| `npm run routes`                                                                                                                      | `ruvyxa routes`                         | route table แบบอ่านง่าย                                                  |
| `npm run routes:json`                                                                                                                 | route JSON command ที่ starter กำหนด    | route output สำหรับเครื่องอ่าน                                           |
| `npm run analyze`                                                                                                                     | `ruvyxa analyze`                        | validate route, import และ server/client boundary                        |
| `npm run analyze:html`                                                                                                                | HTML analysis command ที่ starter กำหนด | หน้าวิเคราะห์แบบ interactive ในเครื่อง                                   |
| `npm run add -- form`                                                                                                                 | `ruvyxa add form`                       | scaffold application flow ที่รองรับ                                      |
| `npm run doctor`, `npm run clean`, `npm run trace -- /`, `npm run bench`, `npm run test:parity`, `npm run plugin -- create my-plugin` | `ruvyxa` command ที่ตรงกัน              | diagnose, ลบ output, ตรวจ route, benchmark, ตรวจ parity หรือสร้าง plugin |

## Application loop ที่แนะนำ

รันจาก root ของ application ที่สร้างแล้ว ไม่ใช่จาก framework monorepo นี้:

```bash
npm run dev
npm run routes
npm run check
npm run build
npm run test:parity
```

ใช้ `npm run routes:json` เมื่อต้องส่งข้อมูล route แบบ structured ให้เครื่องมืออื่น และเปิดรายงานจาก
`npm run analyze:html` เมื่อต้องตรวจ bundle, route, import หรือ boundary `clean` ลบ generated Ruvyxa
build output จึงอย่ารันกับ path ที่มี artifact ที่ดูแลเอง

## การรัน framework CLI จาก monorepo นี้

root ของ repository นี้ตั้งใจมี workspace script เช่น `pnpm build`, `pnpm check` และ `pnpm test` แต่
**ไม่มี** application script เช่น `npm run dev` หรือ `npm run routes` หากต้องการทดสอบ broad fixture
จาก repository root ให้เรียก CLI ผ่าน Cargo และระบุ fixture ให้ชัดเจน:

```bash
cargo run -p ruvyxa_cli -- dev --root examples/demo
cargo run -p ruvyxa_cli -- routes --root examples/demo
cargo run -p ruvyxa_cli -- check --root examples/demo
```

เมื่อดูแล framework เอง ให้รัน `cargo run -p ruvyxa_cli -- <command> --help` CLI ที่ตรวจแล้วมี
`dev`, `build`, `check`, `start`, `preview`, `routes`, `analyze`, `add`, `doctor`, `clean`, `trace`,
`bench`, `test:parity` และ `plugin create`

## Repository script

root `package.json` กำหนด `build`, `check`, `test`, `prepare`, `check:cargo-lock`,
`check:oxc-lockstep`, `format`, `format:check`, `format:staged`, `release:validate`, `release:bump`,
`pack:smoke`, `test:full-flow` และ `publish:dry-run` TypeScript package ที่เผยแพร่กำหนด `build`,
`check`, `test`, `format` และ `prepack` อย่างสม่ำเสมอ; ดู package manifest ที่เกี่ยวข้องสำหรับ test
glob

**ก่อนหน้า:** [การเชื่อมต่อ](09-integrations-auth-data-and-realtime.md) · **ถัดไป:**
[Architecture](11-architecture.md)
