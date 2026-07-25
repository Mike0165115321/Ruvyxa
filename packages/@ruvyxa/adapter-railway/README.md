# @ruvyxa/adapter-railway

Full-stack Railway adapter for Ruvyxa. Railway builds auto-select it through `RAILWAY_PROJECT_ID`;
no `ruvyxa.config.ts` change or separate install is required.

```ts
import { railwayAdapter } from '@ruvyxa/adapter-railway'
import { config } from 'ruvyxa/config'

export default config({ adapter: railwayAdapter() })
```

The build emits a standalone server at `.ruvyxa/deploy/railway/server/index.mjs` and a safe
`railway.json`. Existing project configuration is never overwritten. The server honors Railway's
`PORT` and supports SSR, SSG, CSR, ISR, PPR, API routes, and native realtime.
