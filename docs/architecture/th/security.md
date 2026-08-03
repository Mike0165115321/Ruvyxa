# โมเดลความปลอดภัย (Security Model)

ขอบเขตความปลอดภัยและการบังคับใช้ครอบคลุมการจัดการคำขอ ตัวแปรสภาพแวดล้อม และการทำงานของปลั๊กอิน

---

## การแยกตัวแปรสภาพแวดล้อม (Environment Variable Isolation)

### กฎ

```
RUVYXA_PUBLIC_*  →  ฝังในบันเดิลฝั่งไคลเอ็นต์ มองเห็นได้ในเบราว์เซอร์
ตัวแปรอื่นทั้งหมด   →  เฉพาะเซิร์ฟเวอร์ ไม่ถูกคอมไพล์ลงในไคลเอ็นต์
```

### ชั้นการบังคับใช้

| Layer        | เมื่อใด                                              | กลไก                                                                                                                                     |
| ------------ | ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Graph-level  | `ruvyxa_graph::validate_app()` — หลังค้นพบเส้นทาง    | สแกนซอร์สหา `process.env.<NAME>` และ `process.env['<NAME>']`. ปฏิเสธค่าที่ไม่ใช่ `RUVYXA_PUBLIC_*` ในโมดูลที่ไคลเอ็นต์เข้าถึง → RUV1008  |
| Bundle-level | `ruvyxa_bundler::boundary::check()` — ระหว่างคอมไพล์ | สแกนเดียวกันบนเอาต์พุต JS ที่คอมไพล์แล้ว ตรวจซ้ำหลังทรานส์ฟอร์ม                                                                          |
| Runtime      | การประเมิน `ruvyxa.config.ts`                        | เฉพาะ `RUVYXA_PUBLIC_*` ที่เข้าถึงได้ผ่าน `defineConfig()` เมื่อคอนฟิกถูกประเมินโดยรันไทม์ Node/Bun ที่เลือกสำหรับค่าที่ไคลเอ็นต์มองเห็น |

### การนำไปใช้: `private_env_reads(source)`

สแกนเนอร์ระดับไบต์ที่รู้จำ:

- `process.env.NAME` → จับค่า `NAME`
- `process.env["NAME"]` หรือ `process.env['NAME']` → จับค่า `NAME`

จัดการกับ:

- สตริงลิเทอรัล (ข้ามไป แต่ `${expr}` จะถูกเรียกซ้ำสำหรับเทมเพลตลิเทอรัล)
- เทมเพลตลิเทอรัล (นับความลึกของนิพจน์ที่ซ้อนกัน)
- คอมเมนต์แบบบล็อก `/* */` และคอมเมนต์แบบบรรทัด `//`

ข้อยกเว้น:

- `process.env.NODE_ENV` — อนุญาตเสมอ (การพับค่าตอนบิลด์)
- `process.env.RUVYXA_PUBLIC_*` — อนุญาต (เปิดเผยโดยชัดแจ้ง)

### ตัวอย่างการละเมิด

```typescript
// app/components/secret.tsx — ถูก import โดยหน้าไคลเอ็นต์
const apiKey = process.env.MY_API_KEY // ← RUV1008
```

### วิธีการแก้ไข

```typescript
// ตัวเลือก A: ย้ายไปฝั่งเซิร์ฟเวอร์เท่านั้น
// server/api.ts
const apiKey = process.env.MY_API_KEY

// ตัวเลือก B: ทำให้เป็นสาธารณะ (เฉพาะเมื่อปลอดภัย)
const apiKey = process.env.RUVYXA_PUBLIC_API_KEY
```

---

## ขอบเขตเซิร์ฟเวอร์/ไคลเอ็นต์ (Server/Client Boundary)

การบังคับใช้สองระดับป้องกันไม่ให้โค้ดฝั่งเซิร์ฟเวอร์รั่วไหลเข้าสู่บันเดิลไคลเอ็นต์

### ระดับ 1: การตรวจสอบกราฟ (`ruvyxa_graph::validate_app`)

สแกนซอร์สบนทุกเส้นทางหลังการค้นพบ

| Violation                                        | การตรวจจับ                                                       | รหัส    |
| ------------------------------------------------ | ---------------------------------------------------------------- | ------- |
| `import "server-only"` ในโค้ดที่ไคลเอ็นต์เข้าถึง | สแกนข้อความหา `import "server-only"` หรือ `import 'server-only'` | RUV1007 |
| `process.env.*` ส่วนตัวในกราฟไคลเอ็นต์           | `private_env_reads()` บนซอร์สที่ไคลเอ็นต์เข้าถึงทั้งหมด          | RUV1008 |
| การ import ไดเรกทอรี `server/` ในกราฟไคลเอ็นต์   | เส้นทางไฟล์ขึ้นต้นด้วย `<root>/server/` หลัง canonicalization    | RUV1010 |
| `import "client-only"` ในโค้ดเซิร์ฟเวอร์/API     | สแกนข้อความหา `import "client-only"`                             | RUV1009 |

### ระดับ 2: ขอบเขตบันเดิล (`ruvyxa_bundler::boundary::check`)

ตรวจซ้ำบนเอาต์พุต JS ที่คอมไพล์แล้วหลังทรานส์ฟอร์ม (รูปแบบ re-export อาจเลี่ยงการตรวจสอบเฉพาะซอร์ส)

### กฎไดเรกทอรี `server/`

เฉพาะไดเรกทอรี `server/` ที่รากโปรเจกต์เท่านั้นที่ถูกตรวจสอบ:

```
project/
├── server/          ← ถูกตรวจสอบ: import จากที่นี่ → RUV1010
│   └── db.ts
├── app/
│   └── blog/
│       └── server.ts  ← ไม่ถูกตรวจสอบโดย RUV1010 (ภายในแอป)
```

---

## การตรวจสอบคำขอ (Request Validation)

### การทำให้พาธเป็นรูปแบบมาตรฐาน (Path canonicalization)

`canonical_request_path(path)`:

- แยกด้วย `/`, percent-decode แต่ละเซกเมนต์ (`percent_encoding::percent_decode_str`)
- ปฏิเสธ: เซกเมนต์ว่าง, `.`, `..`, `/` หรือ `\` ที่ถูก decode, อักขระควบคุม (0x00–0x1F)
- ปฏิเสธ: percent encoding ที่ไม่ถูกต้อง (hex ไม่ถูกต้อง, ถูกตัดทอน)

ป้องกัน: path traversal (`/../../../etc/passwd`), null byte injection, CRLF injection

### ขนาดเนื้อหาสูงสุด (Body size limits)

| Endpoint           | คีย์คอนฟิก             | ค่าเริ่มต้น | จุดตรวจสอบ                         |
| ------------------ | ---------------------- | ----------- | ---------------------------------- |
| Action POST        | `security.actionLimit` | กำหนดได้    | ก่อนอ่าน body ใน `action_endpoint` |
| API POST/PUT/etc   | `security.apiLimit`    | กำหนดได้    | ก่อนอ่าน body ใน `handle_request`  |
| การตอบกลับปลั๊กอิน | `security.pluginLimit` | กำหนดได้    | หลังการทำงานของปลั๊กอิน            |

คืนค่า `413 Payload Too Large` เมื่อละเมิด

### การตรวจสอบ Content-Type

เอนด์พอยต์ Action ตรวจสอบ:

- มี header Content-Type และถูกต้อง
- body เป็น JSON ที่ถูกต้อง (ถ้า `application/json`) หรือ form data ที่ถูกต้อง

---

## การป้องกันแหล่งกำเนิดเดียวกัน (Same-Origin Protection)

### เอนด์พอยต์ Action (`same_origin_actions`)

เมื่อ `same_origin_actions: true`:

```rust
fn action_origin_is_cross_site(headers: &HeaderMap, config: &ServerConfig) -> bool {
    let Some(origin) = headers.get("origin").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let expected = format!("http://{}:{}", config.host, config.port);
    origin != expected
}
```

### Sec-Fetch-Metadata (`fetch_metadata_actions`)

เมื่อ `fetch_metadata_actions: true`:

```rust
fn action_fetch_site_is_cross_site(headers: &HeaderMap) -> bool {
    let Some(site) = headers.get("sec-fetch-site").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    site != "same-origin"
}
```

---

## การจำกัดอัตราการเรียก (Rate Limiting)

### สถาปัตยกรรมสองชั้น

| Tier            | ชั้น                     | คอนฟิก                          | คีย์                          |
| --------------- | ------------------------ | ------------------------------- | ----------------------------- |
| HTTP middleware | คำขอทั้งหมด (dev server) | `middleware.builtin.rate_limit` | `"ip"` หรือ `"header:<name>"` |
| เฉพาะ Action    | POST `/__ruvyxa/action`  | `security.actionRateLimit`      | ซับซ้อน: IP + header + path   |

### การนำไปใช้แบบ Sliding-window (`ActionRateLimiter`)

```rust
struct ActionRateLimiter {
    hits: HashMap<String, Vec<Instant>>,   // sliding window
    max_hits: usize,
    window: Duration,
    max_keys: usize,                        // 10,000
}

impl ActionRateLimiter {
    fn allow(&mut self, key: &str) -> bool {
        let hits = self.hits.entry(key.to_string()).or_default();
        // Prune expired hits (older than window)
        hits.retain(|t| t.elapsed() < self.window);
        if hits.len() >= self.max_hits {
            return false;
        }
        hits.push(Instant::now());
        true
    }
}
```

### การจัดการคีย์

- เมื่อถึง 10,000 คีย์ที่ติดตาม: เก็บกวาดทั้งหมดเพื่อลบรายการที่หมดอายุก่อนแทรกรายการใหม่
- รายการที่หมดอายุถูกลบแบบขี้เกียจเมื่อมีการเรียก `allow()`
- header `Retry-After` ถูกส่งคืนเมื่อถูกจำกัดอัตรา: จำนวนวินาทีกว่ารายการที่เก่าที่สุดจะหมดอายุ

### การตอบกลับเมื่อถูกจำกัด

```
HTTP 429 Too Many Requests
Retry-After: <seconds>
```

---

## Security Headers

นำไปใช้เป็น Axum middleware บนทุกการตอบกลับผ่าน `finalize_security_headers()`:

| Header                              | ค่าเริ่มต้นของ Framework                   |
| ----------------------------------- | ------------------------------------------ |
| `X-Content-Type-Options`            | `nosniff`                                  |
| `Referrer-Policy`                   | `strict-origin-when-cross-origin`          |
| `Permissions-Policy`                | `camera=(), microphone=(), geolocation=()` |
| `Cross-Origin-Opener-Policy`        | `same-origin`                              |
| `Cross-Origin-Resource-Policy`      | `same-origin`                              |
| `X-Frame-Options`                   | `DENY`                                     |
| `X-Permitted-Cross-Domain-Policies` | `none`                                     |

ค่าเริ่มต้น `security.headers` คือ `true` ค่าเริ่มต้นจะเติมเฉพาะค่าที่ขาดหาย ดังนั้น header
ที่ชัดเจนจาก `securityHeaders()` หรือ middleware ของแอปพลิเคชันจะมีความสำคัญเหนือกว่า
เมื่อปิดใช้งานค่าเริ่มต้น Ruvyxa
จะลบเฉพาะค่าที่เท่ากับค่าเริ่มต้นของตัวเองและคงนโยบายที่กำหนดไว้ชัดเจน CSP และ HSTS
ไม่ใช่ค่าเริ่มต้น ใช้ `securityHeaders()` หรือการกำหนดค่าในการปรับใช้เมื่อจำเป็น

---

## Trusted Proxy IPs

เมื่ออยู่หลัง reverse proxy `security.trustedProxyIps` กำหนดว่า IP ใดที่เชื่อถือได้สำหรับ:

```rust
fn determine_client_ip(
    config: &ServerConfig,
    remote_addr: SocketAddr,
    headers: &HeaderMap,
) -> SocketAddr {
    if config.trusted_proxy_ips.contains(&remote_addr.ip()) {
        // Trust X-Forwarded-For
        if let Some(forwarded) = headers.get("x-forwarded-for") {
            // Use leftmost (original client) IP
            return parse_first_ip(forwarded);
        }
    }
    remote_addr
}
```

ยังใช้สำหรับ `X-Forwarded-Proto` (การตรวจจับ HTTPS)

---

## รันไทม์ของปลั๊กอิน (Plugin Runtime)

ปลั๊กอิน Ruvyxa เป็นโมดูล JavaScript ของ Node/Bun ไม่ใช่ WASM ปลั๊กอินทำงานในโพรเซส JavaScript
ถาวรเดียวกับตัวเรนเดอร์คอนฟิก Rust ไม่เคยประเมินซอร์สปลั๊กอินโดยตรง

### ขอบเขตการสื่อสาร

1. การตั้งค่าปลั๊กอินทำงานในโพรเซสเรนเดอร์คอนฟิก (การประเมิน `ruvyxa.config.ts`)
2. ฮุกบิลด์ (`build.onResolve`, `build.onTransform`, `build.onComplete`) และฮุก HTTP middleware
   ผ่านโพรเซสย่อย Node/Bun ถาวรเดียวกันผ่าน NDJSON (JSON ที่คั่นด้วยบรรทัดใหม่) ทาง stdin/stdout
3. เนื้อหาคำขอและการตอบกลับที่ข้ามบริดจ์ถูกเข้ารหัส base64
4. เพย์โหลดทั้งหมดถูกจำกัดด้วย `security.pluginLimit` (กำหนดค่าได้) เพื่อป้องกันหน่วยความจำล้น

### คุณสมบัติความปลอดภัย

| Property                | การบังคับใช้                                        |
| ----------------------- | --------------------------------------------------- |
| ไม่มีการประเมินปลั๊กอิน | Rust ไม่เคยประเมินซอร์สปลั๊กอิน; จับคู่เท่านั้น     |
|                         | ผลลัพธ์ฮุกที่มีโครงสร้างจากบริดจ์                   |
| เนื้อหามีขอบเขต         | `plugin_response_body_limit_bytes` บังคับใช้กับทุก  |
|                         | การตอบกลับ middleware ของปลั๊กอิน                   |
| การแยก env ส่วนตัว      | โพรเซสคอนฟิกเข้าถึง env vars; Rust                  |
|                         | ไม่ส่งต่อไปยังบันเดิลไคลเอ็นต์                      |
| การควบคุมเวลา           | หมดเวลาของ Worker pool (`RUVYXA_WORKER_TIMEOUT_MS`) |
|                         | ใช้กับฮุกปลั๊กอินด้วย                               |

---

## ความปลอดภัยของคอนฟิก (Configuration Security)

### การตรวจสอบพาธ

พาธทั้งหมดที่กำหนดค่า (`appDir`, `outDir`, `css.entries[*]`) ต้อง:

- เป็น relative (ไม่มีพาธสัมบูรณ์ → `C:\` หรือ `/`)
- ไม่ข้ามขึ้นไปเหนือรากโปรเจกต์ (ไม่มี `..`)
- ไม่ใช่รากโปรเจกต์เอง

บังคับใช้ใน `ProjectConfig::validate_paths()`

### การตรวจสอบขีดจำกัด

ขีดจำกัดที่กำหนดค่าทั้งหมดต้องอยู่ในขอบเขตที่ปลอดภัย:

- ขีดจำกัด body: `> 0` และ `≤ MAX_BODY_LIMIT`
- ขีดจำกัดอัตรา: `max > 0`, `window > 0`
- ขีดจำกัดปลั๊กอิน: `timeout_ms > 0`, `max_memory > 0`

### ความไม่เปลี่ยนแปลงของคอนฟิก

`#[serde(deny_unknown_fields)]` บน struct คอนฟิกทั้งหมด — ไม่มีค่าเริ่มต้นเงียบสำหรับคำที่พิมพ์ผิด

---

## ความปลอดภัยตอนบิลด์ (Build-Time Security)

### Staging + การ commit แบบอะตอมมิก

บิลด์เขียนไปที่ `out_dir/.ruvyxa-staging-<random>` เมื่อสำเร็จ: เปลี่ยนชื่อ `out_dir` เดิม →
`out_dir.old`, staging → `out_dir`, ลบ `.old` เมื่อล้มเหลว: ทำความสะอาด staging, ผลลัพธ์เดิมคงอยู่

### ไม่มีความลับในผลลัพธ์บิลด์

- บันเดิลไคลเอ็นต์ไม่รวมการอ้างอิง `process.env.<private>` (บังคับใช้ตอนคอมไพล์)
- บันเดิลเซิร์ฟเวอร์มีโค้ดแอปเฉพาะเซิร์ฟเวอร์ (ไม่สามารถเข้าถึงจากเบราว์เซอร์ในโปรดักชัน)
- `build.json` ไม่มีซอร์สโค้ดหรือ env vars

### แฮช dependencies

`config_dependency_hash = blake3(config + config dependencies)`. ใช้สำหรับ:

- คีย์เนมสเปซแคชคอมไพล์
- การตรวจสอบแคช artifacts ที่เรนเดอร์ล่วงหน้า
- การทำให้แคชบิลด์เป็นโมฆะเมื่อคอนฟิกเปลี่ยน
