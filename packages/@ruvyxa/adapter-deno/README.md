# @ruvyxa/adapter-deno

Self-contained Deno runtime adapter for Ruvyxa production builds.

```bash
npm install @ruvyxa/adapter-deno
```

```ts
import { config } from 'ruvyxa/config'
import { denoAdapter } from '@ruvyxa/adapter-deno'

export default config({ adapter: denoAdapter() })
```

Build with `ruvyxa build`, copy `.ruvyxa/deploy/deno/`, then run:

```bash
deno run -A --no-prompt server/index.mjs
```
