# CLI reference

command surface ที่ตรวจสอบแล้วคือ `dev`, `build`, `check`, `start`, `preview`, `routes`, `analyze`,
`add`, `doctor`, `clean`, `trace`, `bench`, `test:parity` และ `plugin create` รัน
`ruvyxa <command> --help` เพื่อดู flag ครบถ้วนของ version ที่ติดตั้ง

| Command                                                                                             | วัตถุประสงค์                                                                      |
| --------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `ruvyxa dev [--root .] [--host H] [--port P] [--runtime node\|bun]`                                 | route watching และ hot reload                                                     |
| `ruvyxa build [--root .] [--target node\|bun\|edge\|static] [--adapter NAME] [--runtime node\|bun]` | production output                                                                 |
| `ruvyxa check` / `analyze`                                                                          | app readiness; route/import/boundary analysis                                     |
| `ruvyxa start` / `preview`                                                                          | serve หรือ preview local ของ build ที่มีอยู่                                      |
| `ruvyxa routes [--json]` / `trace`                                                                  | route table/manifest entry เดียว                                                  |
| `ruvyxa doctor`, `clean`, `bench`, `test:parity`                                                    | diagnose setup, ลบ output, benchmark, เปรียบเทียบ dev/prod route และ smoke render |
| `ruvyxa add`, `ruvyxa plugin create`                                                                | scaffold application flow ที่รองรับ หรือ publishable plugin                       |

## Local loop ที่แนะนำ

```bash
pnpm dev
pnpm routes
pnpm check
pnpm build
pnpm test:parity
```

ใช้ `--root examples/demo` เมื่อรันจาก monorepo root นี้กับ broad fixture `clean` ลบ generated
Ruvyxa build output; อย่ารันกับ path ที่มี artifact ที่ดูแลเอง `analyze --html` มี project script
ที่ตรงกันและสร้าง HTML analysis view

## Repository script

root `package.json` กำหนด `build`, `check`, `test`, `prepare`, `check:cargo-lock`,
`check:oxc-lockstep`, `format`, `format:check`, `format:staged`, `release:validate`, `release:bump`,
`pack:smoke`, `test:full-flow` และ `publish:dry-run` TypeScript package ที่เผยแพร่กำหนด `build`,
`check`, `test`, `format` และ `prepack` อย่างสม่ำเสมอ; ดู package manifest ที่เกี่ยวข้องสำหรับ test
glob

**ก่อนหน้า:** [การเชื่อมต่อ](09-integrations-auth-data-and-realtime.md) · **ถัดไป:**
[Architecture](11-architecture.md)
