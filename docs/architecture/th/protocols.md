# Wire Protocols

สัญญาการสื่อสารระหว่างส่วนประกอบของ Ruvyxa

---

## 1. Node Worker NDJSON Protocol

### การส่งข้อมูล

- **สื่อ**: stdin/stdout pipes
- **รูปแบบ**: newline-delimited JSON (หนึ่ง JSON object ต่อบรรทัด)
- **การเข้ารหัส**: UTF-8
- **การสิ้นสุด**: Node process อ่าน stdin ทีละบรรทัด, เขียน stdout ทีละบรรทัด

### ข้อความคำขอ (`WorkerRequest`)

ข้อความทั้งหมดมีฟิลด์ `"type"` ฟิลด์ร่วม: `"id"` (UUID v4 สำหรับ correlation)

#### SSR Render

```json
{
  "type": "ssr",
  "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "projectRoot": "/Users/project",
  "appDir": "/Users/project/app",
  "pageFile": "/Users/project/app/blog/[slug]/page.tsx",
  "requestPath": "/blog/hello-world",
  "params": { "slug": "hello-world" }
}
```

#### API Route

```json
{
  "type": "api",
  "id": "a1b2c3d4-...",
  "projectRoot": "/Users/project",
  "routeFile": "/Users/project/app/api/search/route.ts",
  "method": "GET",
  "requestPath": "/api/search?q=hello",
  "headers": { "accept": "application/json" },
  "headerPairs": [
    ["accept", "application/json"],
    ["cookie", "sess=abc"]
  ],
  "body": null,
  "bodyBase64": null,
  "streamResponse": true,
  "params": {}
}
```

Headers มีให้ในสองรูปแบบ:

- `headers: HashMap<String, String>` — last-value-wins สำหรับการค้นหาง่าย
- `headerPairs: Vec<(String, String)>` — เก็บค่าทั้งหมดและลำดับ

#### Action

```json
{
  "type": "action",
  "id": "b2c3d4e5-...",
  "projectRoot": "/Users/project",
  "actionFile": "/Users/project/app/action.ts",
  "actionName": "createTodo",
  "payloadJson": "{\"title\":\"Buy milk\"}",
  "contentType": "application/json",
  "requestPath": "/todos"
}
```

#### Client Bundle

```json
{
  "type": "client",
  "id": "c3d4e5f6-...",
  "projectRoot": "/Users/project",
  "appDir": "/Users/project/app",
  "pageFile": "/Users/project/app/page.tsx",
  "requestPath": "/",
  "params": {}
}
```

#### SSG / PPR Render

```json
{
  "type": "ssg",
  "id": "d4e5f6a7-...",
  "projectRoot": "/Users/project",
  "appDir": "/Users/project/app",
  "pageFile": "/Users/project/app/blog/[slug]/page.tsx",
  "requestPath": "/blog/hello-world",
  "params": { "slug": "hello-world" },
  "mode": "ppr",
  "fresh": false
}
```

- `mode`: `"full"` (render สมบูรณ์รวม dynamic) | `"ppr"` (เฉพาะ static shell)
- `fresh`: `true` = ข้าม stale-while-revalidate, render สด

#### Static Params Resolution

```json
{
  "type": "staticParams",
  "id": "e5f6a7b8-...",
  "projectRoot": "/Users/project",
  "pageFile": "/Users/project/app/blog/[slug]/page.tsx",
  "routePath": "/blog/[slug]",
  "segments": ["slug"],
  "routes": [
    { "id": "...", "path": "/posts/[id]", "file": "...", ... }
  ]
}
```

#### Invalidation

```json
{
  "type": "invalidate",
  "id": "f6a7b8c9-...",
  "paths": ["/Users/project/app/components/Button.tsx", "/Users/project/app/page.tsx"]
}
```

#### Ping

```json
{ "type": "ping", "id": "a7b8c9d0-..." }
```

#### Warmup

```json
{
  "type": "warmup",
  "id": "b8c9d0e1-...",
  "projectRoot": "/Users/project",
  "routes": [{ "pageFile": "...", "requestPath": "/", "params": {} }]
}
```

### ข้อความตอบกลับ (`WorkerResponse`)

#### Successful SSR response

```json
{
  "id": "f47ac10b-...",
  "ok": true,
  "html": "<!doctype html><html lang=\"en\"><head>...</head><body>...</body></html>"
}
```

#### Successful API response (ไม่ใช่ streaming)

```json
{
  "id": "a1b2c3d4-...",
  "ok": true,
  "status": 200,
  "headers": { "content-type": "application/json" },
  "headerPairs": [["content-type", "application/json"]],
  "body": "{\"results\":[1,2,3]}"
}
```

#### Successful API response (streaming)

```
{"id":"a1b2c3d4-...","ok":true,"frame":"api-start","status":200,"headers":{"content-type":"text/plain"},"headerPairs":[["content-type","text/plain"]]}
{"id":"a1b2c3d4-...","ok":true,"frame":"api-chunk","bodyBase64":"SGVsbG8="}
{"id":"a1b2c3d4-...","ok":true,"frame":"api-chunk","bodyBase64":"IHdvcmxk"}
{"id":"a1b2c3d4-...","ok":true,"frame":"api-end"}
```

Frames:

- `"api-start"` — stream เริ่มต้น, รวม status + headers
- `"api-chunk"` — body chunk, เข้ารหัส `bodyBase64`
- `"api-end"` — stream สมบูรณ์, terminal

#### Successful action response

```json
{
  "id": "b2c3d4e5-...",
  "ok": true,
  "status": 200,
  "headers": { "content-type": "application/json" },
  "body": "{\"ok\":true,\"id\":42}"
}
```

#### Successful client bundle response

```json
{
  "id": "c3d4e5f6-...",
  "ok": true,
  "script": "var __ruvyxa_shared_modules__=(globalThis.__RUVYXA_SHARED_MODULES__||(globalThis.__RUVYXA_SHARED_MODULES__={}));..."
}
```

#### Successful ping

```json
{ "id": "...", "ok": true, "pong": true }
```

#### Successful warmup

```json
{ "id": "...", "ok": true, "warmed": 42, "moduleCacheSize": 128 }
```

#### Static params response

```json
{
  "id": "...",
  "ok": true,
  "params": [{ "slug": "hello-world" }, { "slug": "about" }]
}
```

#### Error response

```json
{
  "id": "...",
  "ok": false,
  "code": "RUV1100",
  "message": "React SSR failed: Cannot read properties of undefined",
  "stack": "TypeError: Cannot read properties of undefined\n    at Page (file:///...)\n    at renderToString (node:...)"
}
```

ข้อผิดพลาดแบบ streaming ใช้ `frame`:

```json
{
  "id": "...",
  "ok": false,
  "frame": "api-error",
  "message": "Database connection failed",
  "code": "DB_CONN"
}
```

---

## 2. HMR WebSocket Protocol

### การส่งข้อมูล

- **สื่อ**: WebSocket (`ws://` หรือ `wss://`)
- **ทิศทาง**: Server → Browser (ทิศทางเดียว)
- **รูปแบบ**: JSON text frames

### รูปแบบข้อความ

```json
{
  "type": "css-update" | "component-update" | "full-reload",
  "paths": ["/abs/path/to/changed/file.scss", "..."],
  "affectedRoutes": ["/", "/blog/[slug]"],
  "fullReload": false
}
```

### ประเภทเหตุการณ์

| Type                 | Trigger                                         | การทำงานของ Browser                                     |
| -------------------- | ----------------------------------------------- | ------------------------------------------------------- |
| `"css-update"`       | เฉพาะไฟล์ `.css`/`.scss`/`.sass` ที่เปลี่ยน     | แทนที่ `<style data-ruvyxa-css>` ด้วย CSS ที่อัปเดต     |
| `"component-update"` | ไฟล์ component ที่รู้จักเปลี่ยน                 | React Fast Refresh (เรนเดอร์ components ที่เปลี่ยนใหม่) |
| `"full-reload"`      | Layout เปลี่ยน หรือมีการเปลี่ยนแปลงที่ไม่รู้จัก | `window.location.reload()`                              |

### วงจรการเชื่อมต่อ

```
Browser: WebSocket connect to ws://host/__ruvyxa/hmr
Server:  subscribes to reload_tx broadcast channel
         sends JSON on each file change event
Browser: receives message, dispatches to appropriate handler
         auto-reconnects on disconnect (exponential backoff)
```

### ตัวจัดการฝั่ง client (แทรกใน HTML `<script>`)

```javascript
;(function () {
  const protocol = location.protocol === 'https:' ? 'wss' : 'ws'
  const socket = new WebSocket(`${protocol}://${location.host}/__ruvyxa/hmr`)

  socket.addEventListener('message', async (event) => {
    const msg = JSON.parse(event.data)

    if (msg.type === 'css-update') {
      const style = document.querySelector('style[data-ruvyxa-css]')
      if (style) {
        const resp = await fetch(location.href)
        const html = await resp.text()
        const match = html.match(/<style data-ruvyxa-css>([\s\S]*?)<\/style>/)
        if (match) style.textContent = match[1]
      }
    } else if (msg.type === 'component-update') {
      // React Fast Refresh implementation
      if (window.__RUVYXA_FAST_REFRESH__) {
        window.__RUVYXA_FAST_REFRESH__(msg.paths)
      } else {
        location.reload()
      }
    } else {
      location.reload()
    }
  })

  socket.addEventListener('close', () => {
    // Reconnect with backoff
    setTimeout(() => connectHMR(), 1000)
  })
})()
```

---

## 3. Plugin Protocol

Plugins สื่อสารกับ Rust ผ่าน process `runtime/plugin-runtime.mjs` แบบถาวรโดยใช้ newline-delimited
JSON (NDJSON) registry เดียวกันให้บริการทั้ง build hooks และ HTTP middleware

### Build hooks

```json
{ "hook": "build.transform", "code": "...", "id": "/app/page.tsx", "environment": "client" }
```

runtime คืนค่า `{ "ok": true, "result": { "code": "...", "map": "..." } }` หรือ error
ที่มีโครงสร้างประกอบด้วย Ruvyxa diagnostic code, message และ stack

### HTTP middleware

การเรียก request ใช้ ordered header pairs และ base64 bodies ที่เป็นทางเลือก:

```json
{
  "hook": "http.request",
  "request": {
    "method": "POST",
    "path": "/api/items?draft=1",
    "headers": [["content-type", "application/octet-stream"]],
    "bodyBase64": "AAE="
  }
}
```

request hook คืนค่าได้ทั้ง replacement request หรือ response short-circuit response hooks ได้รับ
request และ response ปัจจุบัน และคืนค่า replacement response Rust ตรวจสอบ methods, paths, headers,
status codes และ body limits ก่อนแปลงค่าเป็น Axum types

### วงจรชีวิต

1. CLI หรือ dev server เริ่ม runtime และส่ง `describe`
2. runtime โหลด `ruvyxa.config.ts`, ตรวจสอบชื่อ plugins, และ execute แต่ละ `register` หนึ่งครั้ง
3. Rust ส่ง serialized hook calls ผ่าน process แบบถาวร
4. runtime คืนค่า JSON response หนึ่งตัวต่อ input บรรทัด; diagnostics ไปที่ stderr
5. process ถูกยุติพร้อมกับ owning build หรือ server lifecycle
