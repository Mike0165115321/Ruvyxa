# ตัวอย่างเริ่มต้น Database ORM

Ruvyxa ไม่เลือก database driver และไม่รัน migration แทนแอป ควรวาง driver ไว้ในโมดูลฝั่ง server, ตรวจ
`DATABASE_URL` ตอน build และตั้ง pooling ให้เหมาะกับแพลตฟอร์ม ตัวอย่างต่อไปนี้เป็นรูปแบบขั้นต่ำ
ที่รองรับจริงสำหรับ Prisma และ Drizzle

## Config ร่วม

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'
import { databasePlugin } from '@ruvyxa/database/plugin'

export default config({
  plugins: [databasePlugin({ requiredEnv: ['DATABASE_URL'] })],
})
```

ห้ามใช้ prefix `RUVYXA_PUBLIC_` กับ secret ของฐานข้อมูล เพราะ prefix นี้มีไว้ส่งค่าไป browser

## Prisma ผ่าน Ruvyxa facade

หลังติดตั้ง Prisma สร้าง schema และรัน `prisma generate` ให้สร้าง client ฝั่ง server เพียงจุดเดียว:

```ts
// app/_server/database.ts
import 'server-only'
import { PrismaClient } from '@prisma/client'
import { createDatabase, prismaAdapter } from '@ruvyxa/database'

interface Schema {
  users: { id: string; email: string; name: string | null }
}

const prisma = new PrismaClient()
export const db = createDatabase<Schema>(prismaAdapter(prisma, { models: { users: 'user' } }))
```

เรียก facade นี้จาก Server Component, loader, API route หรือ Server Action:

```ts
// app/users/action.ts
'use server'

import { action } from 'ruvyxa/server'
import { db } from '../_server/database.js'

export const createUser = action
  .input({ parse: (value) => ({ email: String(value.email) }) })
  .handler(({ input }) => db.users.create({ data: input }))
```

Facade แปลง typed CRUD contract ของ Ruvyxa ไปยัง Prisma delegate ส่วน connection, schema และ
migration ยังเป็นหน้าที่ของ Prisma

## Drizzle starter

Drizzle มี typed SQL query อยู่แล้ว จึงควรใช้ client โดยตรง แทนการซ่อน SQL semantics ไว้หลัง generic
adapter ที่รองรับไม่ครบ:

```ts
// app/_server/schema.ts
import { pgTable, text, uuid } from 'drizzle-orm/pg-core'

export const users = pgTable('users', {
  id: uuid('id').defaultRandom().primaryKey(),
  email: text('email').notNull().unique(),
})
```

```ts
// app/_server/database.ts
import 'server-only'
import { drizzle } from 'drizzle-orm/postgres-js'
import postgres from 'postgres'

import * as schema from './schema.js'

const connection = postgres(process.env.DATABASE_URL!, { prepare: false })
export const db = drizzle(connection, { schema })
```

```tsx
// app/users/page.tsx
import { db } from '../_server/database.js'

export default async function UsersPage() {
  const users = await db.query.users.findMany()
  return (
    <ul>
      {users.map((user) => (
        <li key={user.id}>{user.email}</li>
      ))}
    </ul>
  )
}
```

ให้รัน `drizzle-kit generate` และ `drizzle-kit migrate` ใน CI หรือ release job ไม่ใช่ภายใน request
ถ้าใช้ serverless database ให้เลือก pooled/serverless driver ของผู้ให้บริการ ส่วน Node process
ที่ทำงานยาวให้ reuse driver หนึ่ง instance ต่อ process

`@ruvyxa/database` รับ custom `DatabaseAdapter` ได้ แต่ adapter ต้องแปลงทุก operation ที่ประกาศว่า
รองรับอย่างครบถ้วน ไม่ควรนำ adapter ตัวอย่างที่รองรับเพียงบาง query ไปใช้เป็น ORM bridge ทั่วไป

## Checklist ก่อน deploy

- เก็บโมดูลฐานข้อมูลใต้ `app/_server/`, `server/` หรือป้องกันด้วย `server-only`
- รัน migration ก่อนส่ง traffic ไป release ที่ต้องใช้ schema ใหม่
- ตั้ง connection pooling ให้ตรงกับ driver และแพลตฟอร์ม
- ใช้ mock จาก `@ruvyxa/testing` สำหรับ unit test และใช้ ORM จริงกับฐานข้อมูลชั่วคราวสำหรับ
  integration test
- ปิด client ในสคริปต์แบบ one-shot; process แบบ long-running/serverless ให้ทำตาม lifecycle ของ
  driver
