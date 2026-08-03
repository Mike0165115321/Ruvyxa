# Database ORM starters

Ruvyxa does not choose a database driver or own migrations. Keep the driver in a server-only module,
validate `DATABASE_URL` during the build, and let the deployment platform provide pooling. The
following starters are the smallest supported Prisma and Drizzle integrations.

## Shared configuration

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'
import { databasePlugin } from '@ruvyxa/database/plugin'

export default config({
  plugins: [databasePlugin({ requiredEnv: ['DATABASE_URL'] })],
})
```

Never rename a database secret with the `RUVYXA_PUBLIC_` prefix. That prefix deliberately makes a
value browser-readable.

## Prisma with the Ruvyxa facade

Install Prisma, create its schema, run `prisma generate`, then expose one server-only client:

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

Use the facade from a Server Component, loader, API route, or Server Action:

```ts
// app/users/action.ts
'use server'

import { action } from 'ruvyxa/server'
import { db } from '../_server/database.js'

export const createUser = action
  .input({ parse: (value) => ({ email: String(value.email) }) })
  .handler(({ input }) => db.users.create({ data: input }))
```

The facade maps Ruvyxa's typed CRUD contract to Prisma delegates. Prisma remains responsible for
connections, schema generation, and migrations.

## Drizzle starter

Drizzle already exposes typed SQL queries, so use its client directly rather than hiding SQL
semantics behind an incomplete generic adapter:

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

Use `drizzle-kit generate` and `drizzle-kit migrate` in CI or a release job, not inside a request.
For a serverless database, use the provider's pooled/serverless driver; for a long-lived Node
process, reuse one driver instance per process. `@ruvyxa/database` still accepts a complete custom
`DatabaseAdapter`, but an adapter must translate every operation it claims to support—do not ship a
partial translation as a general-purpose ORM bridge.

## Deployment checklist

- Keep database modules under `app/_server/`, `server/`, or behind `server-only`.
- Run migrations before routing production traffic to a schema-dependent release.
- Configure connection pooling for the selected adapter and platform.
- Use `@ruvyxa/testing` mocks for unit tests; use the real ORM against a disposable database for
  integration tests.
- Disconnect clients in one-shot scripts. Long-running and serverless processes should follow the
  selected driver's lifecycle guidance.
