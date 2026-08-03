# คู่มือรหัส Diagnostic

ทุกข้อผิดพลาดของ framework มีรหัสโครงสร้าง `RUV####` พร้อมชื่อเรื่อง, คำอธิบาย, ตำแหน่ง source,
และคำแนะนำการแก้ไข กำหนดใน `ruvyxa_diagnostics` และใช้โดยทุก crate

> **คู่มือกู้คืนฉบับปัจจุบัน:** [การจัดการข้อผิดพลาด](../../guides/th/16-error-handling.md) คือ
> แค็ตตาล็อกรหัส RUV ที่ครบถ้วนและเน้นการใช้งานของแอป ครอบคลุม error.tsx, notFound(), API responses,
> actions, client loaders, hydration reporting รวมถึงรหัสส่งต่อ/สงวนและ test-only
> หน้านี้จึงคงไว้สำหรับอธิบายสถาปัตยกรรมของ diagnostic

---

## โครงสร้าง Diagnostic

```rust
pub struct Diagnostic {
    pub code: &'static str,            // "RUV1007"
    pub title: &'static str,           // บรรทัดเดียวที่มนุษย์อ่านเข้าใจ
    pub explanation: String,           // เกิดอะไรขึ้นและทำไม
    pub span: Option<SourceSpan>,      // file:line:col
    pub import_chain: Vec<String>,     // ร่องรอยสำหรับ boundary violations
    pub suggested_fix: String,         // ข้อความแนะนำที่นำไปแก้ไขได้
    pub affected_routes: Vec<String>,  // route IDs ที่ได้รับผล
}

pub struct SourceSpan {
    pub file: PathBuf,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

pub enum RuvyxaError {
    Diagnostic(Box<Diagnostic>),
    Io { message: String, source: Option<Arc<std::io::Error>> },
    Message(String),
}

pub type Result<T> = std::result::Result<T, RuvyxaError>;
```

---

## Graph Diagnostics (RUV1xxx)

ตรวจพบโดย `ruvyxa_graph`

| รหัส        | ชื่อเรื่อง                | เงื่อนไข                                                                                     | วิธีแก้                                                  |
| ----------- | ------------------------- | -------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| **RUV1001** | ไม่พบไดเรกทอรี app        | ไม่มีไดเรกทอรี `app/` ที่ root ของโปรเจกต์                                                   | สร้างไดเรกทอรี `app/` หรือตั้งค่า `appDir` ใน config     |
| **RUV1002** | Segment เส้นทางไม่ถูกต้อง | ไวยากรณ์ dynamic segment ผิด: `[a b]`, `[]`, `[.name]`, วงเล็บใน plain text segment          | ใช้ `[param]`, `[...rest]`, หรือ `[[...rest]]`           |
| **RUV1003** | เส้นทางขัดแย้งกัน         | สอง routes map ไปยัง match shape เดียวกัน (เช่น `/blog/[slug]` และ `/blog/[id]` → `/blog/:`) | ทำให้ path ต่างกันด้วย static prefix segments ที่ไม่ซ้ำ  |
| **RUV1004** | ขาด default export        | ไฟล์ page component ไม่มี `export default`                                                   | เพิ่ม `export default function Page() { ... }` ในไฟล์เพจ |

## Boundary Diagnostics (RUV1xxx)

ตรวจพบโดย `ruvyxa_graph` และ `ruvyxa_bundler`

| รหัส        | ชื่อเรื่อง                              | เงื่อนไข                                                             | วิธีแก้                                                                                               |
| ----------- | --------------------------------------- | -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| **RUV1007** | โมดูล server-only ใน client graph       | ตรวจพบ `import "server-only"` ในโมดูลที่ client เข้าถึงได้           | ย้าย server logic ไปยังไดเรกทอรี `server/`, `action.ts`, หรือลบ server-only import ออกจาก client code |
| **RUV1008** | ตัวแปรสภาพแวดล้อมส่วนตัวใน client graph | `process.env.VARIABLE` (ไม่ใช่ `RUVYXA_PUBLIC_*`) ใน client bundle   | เปลี่ยนชื่อเป็น `RUVYXA_PUBLIC_VARIABLE` หรือย้าย env read ไปยัง server-only code                     |
| **RUV1009** | โมดูล client-only ใน server graph       | `import "client-only"` ในโมดูล API/server                            | ลบ client-only dependency ออกจาก server-side code                                                     |
| **RUV1010** | โมดูล server directory ใน client graph  | ไฟล์ภายใต้ไดเรกทอรี `server/` ถูก import โดย client-reachable module | ย้าย logic ที่ importable ออกจากไดเรกทอรี `server/` หรือเก็บ imports ไว้ฝั่ง server                   |

## Server Runtime Diagnostics (RUV11xx–RUV16xx)

ตรวจพบโดย `ruvyxa_dev_server`

### ข้อผิดพลาด SSR

| รหัส        | ชื่อเรื่อง         | เงื่อนไข                                              | วิธีแก้                                                           |
| ----------- | ------------------ | ----------------------------------------------------- | ----------------------------------------------------------------- |
| **RUV1100** | React SSR ล้มเหลว  | `renderToString()` เกิดข้อผิดพลาดใน JavaScript worker | ตรวจสอบ component, imports ที่ขาด, JSX ไม่ถูกต้อง                 |
| **RUV1102** | ไม่พบ SSR renderer | `runtime/ssr-renderer.mjs` หายไปใน JavaScript workers | ติดตั้งแพ็กเกจ `ruvyxa` ใหม่หรือตรวจสอบว่า runtime scripts ถูกรวม |

### ข้อผิดพลาด API

| รหัส        | ชื่อเรื่อง                 | เงื่อนไข                                                   | วิธีแก้                                             |
| ----------- | -------------------------- | ---------------------------------------------------------- | --------------------------------------------------- |
| **RUV1200** | การทำงาน API route ล้มเหลว | API handler เกิดข้อผิดพลาดที่ไม่ได้จับใน JavaScript worker | ตรวจสอบ API route, รูปร่าง request ตรงตามที่คาดหวัง |
| **RUV1201** | ไม่มีพอร์ตเซิร์ฟเวอร์ว่าง  | ไม่สามารถผูกพอร์ตหลังจากลอง 100 ครั้ง                      | ปล่อยพอร์ตในช่วงหรือเปลี่ยนพอร์ตที่ตั้งค่า          |
| **RUV1202** | ไม่พบ API renderer         | `runtime/api-renderer.mjs` หายไป                           | ติดตั้ง `ruvyxa` ใหม่                               |

### ข้อผิดพลาด Client Bundle

| รหัส        | ชื่อเรื่อง                    | เงื่อนไข                                       | วิธีแก้                                                      |
| ----------- | ----------------------------- | ---------------------------------------------- | ------------------------------------------------------------ |
| **RUV1300** | Client bundling ล้มเหลว       | ข้อผิดพลาดการคอมไพล์ระหว่าง dev client request | ตรวจสอบไฟล์ page และ imports ว่ามีข้อผิดพลาด                 |
| **RUV1303** | ไม่พบ client route            | เส้นทางที่ร้องขอไม่อยู่ใน manifest             | ตรวจสอบว่า route file มีอยู่และใช้ naming convention ถูกต้อง |
| **RUV1304** | Client bundle สำหรับ non-page | ร้องขอ client bundle สำหรับ API route          | API routes ไม่มี client bundles; มีเฉพาะ page routes         |

### ข้อผิดพลาด Style

| รหัส        | ชื่อเรื่อง                       | เงื่อนไข                                                        | วิธีแก้                                                |
| ----------- | -------------------------------- | --------------------------------------------------------------- | ------------------------------------------------------ |
| **RUV1400** | Tailwind CSS compilation ล้มเหลว | โปรเซส Tailwind CLI ล้มเหลวระหว่าง build                        | แก้ไข Tailwind config/CSS แล้ว build ใหม่              |
| **RUV1401** | ไม่พบ Tailwind CSS CLI           | ไม่พบไบนารี `tailwindcss` สำหรับโปรเจกต์ที่ตั้งค่า Tailwind ไว้ | ติดตั้ง dependency ของ Tailwind CLI                    |
| **RUV1402** | Sass compilation ล้มเหลว         | `grass` Sass compiler ล้มเหลว (syntax error, import not found)  | แก้ไข Sass syntax หรือตรวจสอบว่า imported files มีอยู่ |
| **RUV1403** | ไม่สามารถหา Stylesheet import    | CSS `@import` หรือ Sass `@use` มี path ที่แก้ไม่ได้             | ใช้ relative path ที่ถูกต้องหรือติดตั้ง dependency     |
| **RUV1404** | CSS entry อยู่นอก project root   | path ใน `css.entries` ที่ตั้งค่าไว้ resolve ออกนอก project root | ชี้ entry ไปยัง path ที่อยู่ภายใน project root         |

### ข้อผิดพลาด SSG / ISR / Action / PPR

| รหัส        | ชื่อเรื่อง         | เงื่อนไข                                     | วิธีแก้                                             |
| ----------- | ------------------ | -------------------------------------------- | --------------------------------------------------- |
| **RUV1500** | SSG render ล้มเหลว | การเรนเดอร์ static generation เกิดข้อผิดพลาด | ตรวจสอบ page component ว่ามี runtime errors         |
| **RUV1501** | ไม่พบ Action file  | ไฟล์ server action handler หายไป             | สร้างไฟล์ `action.ts` หรือแก้ไขชื่อ action          |
| **RUV1550** | PPR render ล้มเหลว | Partial pre-rendering ล้มเหลว                | ตรวจสอบข้อผิดพลาดในส่วน static หรือ dynamic ของหน้า |

### ข้อผิดพลาด Config Validation

| รหัส        | ชื่อเรื่อง            | เงื่อนไข                     | วิธีแก้                                     |
| ----------- | --------------------- | ---------------------------- | ------------------------------------------- |
| **RUV1600** | Config load ล้มเหลว   | ข้อผิดพลาดการเรนเดอร์ config | ตรวจสอบ ruvyxa.config.ts syntax และ runtime |
| **RUV1601** | ค่า Config น้อยเกินไป | ค่าจำกัด ≤ 0                 | ตั้งค่าบวกสำหรับ body limit, rate limit ฯลฯ |
| **RUV1602** | ค่า Config มากเกินไป  | ค่าจำกัดเกิน MAX             | ลดค่าให้อยู่ในขอบเขตที่อนุญาต               |

## Middleware และ Plugin Diagnostics (RUV2xxx)

ตรวจพบโดย `ruvyxa_middleware`

| รหัส        | ชื่อเรื่อง                      | เงื่อนไข                                                                                | วิธีแก้                                                                             |
| ----------- | ------------------------------- | --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| **RUV2000** | ข้อผิดพลาดการตั้งค่า Middleware | การตั้งค่า middleware ไม่ถูกต้อง (ชื่อ header ไม่ดี, CORS ไม่เข้ากัน, rate limit ติดลบ) | แก้ไขค่า config ตามข้อความ validation error                                         |
| **RUV2001** | Middleware execution ล้มเหลว    | Tower middleware layer panic หรือคืน error                                              | ตรวจสอบการ implement custom middleware, ตรวจสอบ dependencies                        |
| **RUV2100** | ข้อผิดพลาด Plugin runtime       | Plugin runtime เริ่มไม่ได้หรือคืนข้อมูลโปรโตคอลไม่ถูกต้อง                               | ตรวจสอบ Node/Bun runtime และการตั้งค่าปลั๊กอิน                                      |
| **RUV2101** | ข้อผิดพลาด Plugin hook          | Callback ของปลั๊กอินเกิดข้อผิดพลาดหรือคืนค่าที่ไม่รองรับ                                | ตรวจสอบ hook ที่ระบุและคืน `undefined`, `Request`, หรือ `Response` ตามที่เอกสารระบุ |

## Official Package Diagnostics (RUV3xxx)

ตรวจพบโดยแพ็กเกจทางการระหว่าง adapter calls, authentication requests, และ builds

| รหัส             | แพ็กเกจ            | เงื่อนไข                                                                | วิธีแก้                                                           |
| ---------------- | ------------------ | ----------------------------------------------------------------------- | ----------------------------------------------------------------- |
| **RUV3001**      | `@ruvyxa/database` | โมเดล, การดำเนินการ, อาร์กิวเมนต์, หรือ environment ที่จำเป็นไม่ถูกต้อง | แก้ไข query/options และเก็บ database environment vars เป็นส่วนตัว |
| **RUV3002**      | `@ruvyxa/database` | Adapter ไม่มี mapped model หรือ operation                               | แก้ไข Prisma model mapping หรือ implement transport operation     |
| **RUV3003**      | `@ruvyxa/database` | ขอ Transaction จาก adapter ที่ไม่มี transaction                         | ใช้ adapter ที่รองรับ atomic transaction                          |
| **RUV3100–3104** | `@ruvyxa/auth`     | Auth runtime, request, rate limit, token, หรือ provider ล้มเหลว         | ตรวจสอบ provider/store ที่ระบุและรักษา atomic store semantics     |
| **RUV3105**      | `@ruvyxa/auth`     | ใช้ process-local auth store ใน production build                        | กำหนดค่า durable session/token และ rate-limit stores              |
| **RUV3201**      | `@ruvyxa/realtime` | เลือก native realtime สำหรับ deployment ที่ไม่รองรับ                    | Self-host ด้วย Node/Bun หรือลบ native realtime plugin             |

---

## ช่วงรหัส Diagnostic

| ช่วง    | Crate ต้นทาง        | หมวดหมู่                              |
| ------- | ------------------- | ------------------------------------- |
| RUV10xx | `ruvyxa_graph`      | Route discovery และ validation        |
| RUV11xx | `ruvyxa_dev_server` | SSR rendering                         |
| RUV12xx | `ruvyxa_dev_server` | API และ server                        |
| RUV13xx | `ruvyxa_dev_server` | Client bundles                        |
| RUV14xx | `ruvyxa_dev_server` | Styles                                |
| RUV15xx | `ruvyxa_dev_server` | SSG/ISR/Actions/PPR                   |
| RUV16xx | `ruvyxa_dev_server` | Config validation                     |
| RUV20xx | `ruvyxa_middleware` | Middleware config และ execution       |
| RUV21xx | `ruvyxa_middleware` | Plugin bridge                         |
| RUV30xx | `@ruvyxa/database`  | Database adapter และ query validation |
| RUV31xx | `@ruvyxa/auth`      | Authentication และ store safety       |
| RUV32xx | `@ruvyxa/realtime`  | Realtime deployment compatibility     |

---

## การเพิ่ม Diagnostic ใหม่

ฟิลด์ที่ต้องมี:

1. **รหัส**: เลือกรหัสถัดไปที่ว่างในช่วงที่ถูกต้อง
2. **ชื่อเรื่อง**: บรรทัดเดียวกระชับอธิบายการละเมิด
3. **คำอธิบาย**: ข้อกำหนดคืออะไรและทำไมถูกละเมิด
4. **Span**: ตำแหน่งไฟล์เมื่อทราบ (ใช้ `SourceSpan::from_path` ถ้าไม่มี line/column)
5. **คำแนะนำการแก้ไข**: คำแนะนำที่นำไปปฏิบัติได้จริง ใช้ `format!()` สำหรับค่า dynamic

ตัวอย่าง:

```rust
Diagnostic::new(
    "RUV1010",
    "มีไดเรกทอรี Server ใน client graph",
    format!("ไฟล์ '{}' อยู่ในไดเรกทอรี server/ แต่ reachable จาก client code '{}'", server_file.display(), entry),
    SourceSpan::from_path(&entry),
    format!("ย้าย shared logic ออกจาก server/ ไปยัง shared module หรือเก็บ imports ของ '{}' ไว้เฉพาะไฟล์ server/API", server_file.display()),
)
```

เพิ่ม tests สำหรับ diagnostic ใหม่ ถ้าผู้ใช้ต้องดำเนินการเพื่อกู้คืน ให้อัปเดต `docs/guides/`
ด้วยรหัสข้อผิดพลาดและขั้นตอนการแก้ไข
