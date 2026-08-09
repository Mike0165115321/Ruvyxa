# @ruvyxa/adapter-render

Full-stack Render adapter for Ruvyxa. Render builds auto-select it through `RENDER=true`.

```ts
import { render } from '@ruvyxa/adapter-render'
import { config } from 'ruvyxa/config'

export default config({ adapter: render() })
```

The build emits `.ruvyxa/deploy/render/server/index.mjs` plus a Render Blueprint. Existing
`render.yaml` files are never overwritten. The server honors `PORT`, binds to `0.0.0.0`, and
supports SSR, SSG, CSR, ISR, PPR, API routes, and native realtime.
