import { definePlugin } from '@ruvyxa/core/plugin'
import type { RuvyxaPlugin } from '@ruvyxa/core/plugin'

import { DatabaseAdapterError } from './adapters.js'

export interface DatabasePluginOptions {
  /** Private environment variables that must exist for production builds. */
  requiredEnv?: readonly string[]
}

/** Validate database deployment requirements through the Ruvyxa build socket. */
export function databasePlugin(options: DatabasePluginOptions = {}): RuvyxaPlugin {
  const names = [...new Set(options.requiredEnv ?? [])]
  for (const [index, name] of names.entries()) {
    if (!/^[A-Z_][A-Z0-9_]*$/.test(name)) {
      throw new TypeError(`databasePlugin() requiredEnv[${index}] is not a valid variable name`)
    }
    if (name.startsWith('RUVYXA_PUBLIC_')) {
      throw new TypeError(`databasePlugin() refuses public database variable ${name}`)
    }
  }
  return definePlugin({
    name: 'ruvyxa:database',
    register({ build }) {
      build.onComplete(() => {
        const missing = names.filter((name) => !process.env[name]?.trim())
        if (missing.length > 0) {
          throw new DatabaseAdapterError(
            'RUV3001',
            `missing private database environment variables: ${missing.join(', ')}`,
          )
        }
      })
    },
  })
}
