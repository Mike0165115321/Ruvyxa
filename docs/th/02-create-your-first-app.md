# สร้าง Ruvyxa app แรกของคุณ

## สร้าง application

workspace เผยแพร่ `create-ruvyxa` และ source ของมันมี template `minimal`, `blog`, `crud` และ
`api-backend` ใช้ generator เพื่อเริ่มต้น project ที่สมบูรณ์และไม่ผูกกับ package manager
รายใดรายหนึ่ง

```bash
pnpm create ruvyxa my-app
cd my-app
pnpm install
pnpm dev
```

script ใน project ที่สร้างจะเรียก binary `ruvyxa` ที่ติดตั้งไว้ `dev` จะค้นหา route และเริ่ม hot
reload; root เริ่มต้นคือ current directory เปิด URL ที่คำสั่งแสดง (ค่า server ปริยายคือ
`localhost:3000` หากไม่ override)

## ติดตั้งใน React project เดิม

template ยืนยัน dependency ขั้นต่ำด้านล่าง ควรรักษา React version ให้เข้ากันทั้งชุด

```bash
pnpm add ruvyxa @ruvyxa/react react react-dom
pnpm add -D typescript @types/react @types/react-dom
```

สร้าง `ruvyxa.config.ts`:

```ts
import { config } from 'ruvyxa/config'

export default config({
  appDir: 'app',
  outDir: '.ruvyxa',
  server: { host: 'localhost', port: 3000 },
})
```

จากนั้นเพิ่มไฟล์ตาม [โครงสร้างโปรเจกต์](03-project-structure.md) อย่าใส่ secret ในตัวแปร
`RUVYXA_PUBLIC_`: prefix นี้ถูกเปิดเผยให้ browser code โดยตั้งใจ

## สร้าง vertical slice ที่ทำงานได้จริง

หลังติดตั้ง dependency แล้ว ให้สร้างไฟล์เหล่านี้ ตัวอย่างนี้ตั้งใจให้เล็ก: มันพิสูจน์ page routing,
layout และ API route ก่อนที่จะเพิ่ม database, auth หรือ plugin

```text
app/
├── layout.tsx
├── page.tsx
└── api/
    └── health/
        └── route.ts
```

```tsx
// app/layout.tsx
import type { ReactNode } from 'react'

export const meta = { title: 'My Ruvyxa app', description: 'First Ruvyxa app' }

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  )
}
```

```tsx
// app/page.tsx
export default function Home() {
  return (
    <main>
      <h1>Ruvyxa is running</h1>
      <p>Edit app/page.tsx and save.</p>
    </main>
  )
}
```

```ts
// app/api/health/route.ts
export function GET() {
  return Response.json({ status: 'ok' })
}
```

รัน `pnpm dev`, เปิด `/` แล้วเปิด `/api/health` request แรก render page; health route คืน JSON ที่มี
`status: "ok"` บันทึกการแก้ใน `app/page.tsx` เพื่อยืนยัน hot reload แล้วตรวจ discovery และ
production behavior:

```bash
pnpm routes
pnpm check
pnpm build
pnpm test:parity
```

หากคำสั่งใดล้มเหลว ให้หยุดที่คำนั้นและใช้ [Troubleshooting](16-troubleshooting-upgrades.md) ก่อน
deploy `test:parity` เปรียบเทียบ dev/prod route และ smoke-render page route; มันไม่แทน application
test

## Scripts

```json
{
  "scripts": {
    "dev": "ruvyxa dev",
    "build": "ruvyxa build",
    "start": "ruvyxa start",
    "preview": "ruvyxa preview",
    "check": "ruvyxa check",
    "routes": "ruvyxa routes"
  }
}
```

`start` และ `preview` ใช้ production build ที่มีอยู่ จึงต้องรัน `build` ก่อน `check` คือคำสั่ง
readiness ระดับ application ดู flag ที่ยืนยันแล้วใน [CLI reference](10-cli.md)

**ก่อนหน้า:** [บทนำ](01-introduction.md) · **ถัดไป:** [โครงสร้างโปรเจกต์](03-project-structure.md)
