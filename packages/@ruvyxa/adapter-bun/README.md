# @ruvyxa/adapter-bun

Bun runtime adapter for Ruvyxa production builds.

## Install

```bash
npm install @ruvyxa/adapter-bun
```

## Usage

```ts
import { config } from 'ruvyxa/config'
import { bun } from '@ruvyxa/adapter-bun'

export default config({
  adapter: bun(),
})
```

## Deployment Artifact

```json
{
  "name": "bun",
  "target": "node",
  "platform": "bun",
  "entry": ".ruvyxa/server/app",
  "assetsDir": ".ruvyxa/assets",
  "clientDir": ".ruvyxa/client",
  "chunkManifest": ".ruvyxa/client/chunk-manifest.json"
}
```

`ruvyxa build` creates a self-contained server. Run it without the Ruvyxa CLI:

```bash
bun .ruvyxa/deploy/bun/server/index.mjs
```

The generated server streams SSR/API response bodies and honors `PORT` and `HOST`.
