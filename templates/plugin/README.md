# ruvyxa-plugin-**PLUGIN_NAME**

A TypeScript/JavaScript plugin for Ruvyxa.

## Start

```bash
npm install
npm test
```

Edit `src/index.ts`. The generated example adds `x-__PLUGIN_NAME__: active` to every response.

## Use in an app

```ts
import { config } from 'ruvyxa/config'
import __PLUGIN_IDENTIFIER__ from 'ruvyxa-plugin-__PLUGIN_NAME__'

export default config({ plugins: [__PLUGIN_IDENTIFIER__] })
```

The plugin can combine HTTP, build, development, diagnostics, and native capability sockets in one
`register()` function. See the Ruvyxa plugin guide for every socket and complete examples.

## Publish

```bash
npm publish
```
