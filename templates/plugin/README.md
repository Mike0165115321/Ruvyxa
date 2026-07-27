# ruvyxa-plugin-**PLUGIN_NAME**

A TypeScript/JavaScript plugin for Ruvyxa.

## Start

```bash
npm install
npm test
```

Edit `src/index.ts`. The generated example adds `x-__PLUGIN_NAME__: active` to every response with
the concise `headers` declaration. Add `http`, `build`, `dev`, `diagnostics`, or `native` sections
only when the plugin needs them; use `register()` for advanced composition or repeated hooks.

## Use in an app

```ts
import { config } from 'ruvyxa/config'
import __PLUGIN_IDENTIFIER__ from 'ruvyxa-plugin-__PLUGIN_NAME__'

export default config({ plugins: [__PLUGIN_IDENTIFIER__] })
```

See the Ruvyxa plugin guide for concise sections, advanced sockets, and complete examples.

## Publish

```bash
npm publish
```
