# Troubleshooting และ compatibility เมื่ออัปเกรด

รัน diagnostic ที่แคบที่สุดก่อน จาก application root:

```bash
pnpm routes
pnpm check
pnpm analyze
pnpm doctor
pnpm trace --help
pnpm test:parity
```

## อาการและวิธีแก้ที่มีหลักฐานรองรับ

| อาการ                                         | เงื่อนไขที่เป็นไปได้                                                                          | ตรวจและแก้                                                                                  |
| --------------------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| route หาย                                     | ไฟล์ไม่ตาม special-file/dynamic-segment rule ที่ค้นหา                                         | รัน `routes`; เปรียบเทียบ directory/name กับ [โครงสร้างโปรเจกต์](03-project-structure.md)   |
| client build รายงาน private import/env access | boundary validation พบ server-only import หรือ environment value ที่ไม่ public ใน client path | ย้ายงานไป server-side; เปิดเผยเฉพาะ `RUVYXA_PUBLIC_*` ที่ปลอดภัยโดยเจตนา                    |
| static build ล้มเหลว                          | static adapter ไม่มี generated prerender page หรือ route ต้องใช้ runtime-only behavior        | ใช้ target ที่เข้ากัน หรือให้ static param/route strategy; ตรวจ build output                |
| `RUV2102`                                     | plugin definition ไม่มี name/behavior หรือ hook shape ไม่ถูกต้อง                              | ให้ `definePlugin` มี `name` ไม่ว่างและ declaration/register callback ที่ถูกต้อง            |
| `RUV3001`–`RUV3003`                           | database adapter input, mapping หรือ operation ทำไม่ได้                                       | ตรวจ `DatabaseAdapterError` message และ model/table mapping ของ adapter                     |
| `RUV3201`                                     | native realtime build สำหรับ target/adapter ที่ไม่รองรับ                                      | deploy long-lived Node/Bun output หรือเอา realtime ออก                                      |
| action/API ปฏิเสธ body                        | body เกิน action/API limit ที่ตั้ง หรือ input parser throw                                    | ดู `security.actionLimit`/`apiLimit`; validate และคืน application error ที่ปลอดภัย          |
| cache ดูเก่า                                  | entry ยังใน TTL/SWR หรืออีก process มี memory cache ของตน                                     | ใช้ `invalidateCache`, ตรวจ strategy และใช้ shared infrastructure สำหรับข้อมูลหลาย instance |

## คำถามที่พบบ่อย

**ทำไม route 404 หลังเรียก `notFound`?** `@ruvyxa/react` throw tagged signal และ route boundary
ที่ใกล้ที่สุด render `not-found.tsx` ส่วน `ruvyxa/server` คืน 404 response ให้ import version
ที่เหมาะกับ page rendering หรือ HTTP handler

**ทำไม environment value หายจาก browser?** มีเพียง `RUVYXA_PUBLIC_*` ที่ตั้งใจให้ client ใช้ ย้าย
secret หรือ server-only computation ออกจาก client code แทนการเปลี่ยน prefix

**อัปเกรดได้โดยไม่มี migration guide ไหม?** repository มี `CHANGELOG.md` แต่เอกสารนี้ไม่อนุมาน
migration ราย version จากมัน ก่อนอัปเกรดให้เปรียบเทียบ export/config type แล้วรัน `pnpm check`,
`pnpm build` และ `pnpm test:parity` กับ app ของคุณ ใช้ `Seo.card` แทน `Seo.twitterCard` ซึ่งเป็น
migration ที่เป็นรูปธรรม

**ก่อนหน้า:** [Deploy, run และ operate ใน production](15-deploy-run-and-operate.md) · **ถัดไป:**
[Public API reference](17-public-api-reference.md)
