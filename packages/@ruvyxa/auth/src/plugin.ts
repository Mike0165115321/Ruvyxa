import { definePlugin } from '@ruvyxa/core/plugin'
import type { RuvyxaPlugin } from '@ruvyxa/core/plugin'

export interface AuthPluginBridge {
  basePath: string
  handle(request: Request): Promise<Response | undefined>
  validateBuild(manifest: Readonly<Record<string, unknown>>): void
}

/** Build the auth runtime's framework plugin from its isolated request handler. */
export function createAuthPlugin(bridge: AuthPluginBridge): RuvyxaPlugin {
  return definePlugin({
    name: 'ruvyxa:auth',
    register({ http, build }) {
      http.onRequest({
        match: [`${bridge.basePath}/*`],
        handler: ({ request }) => bridge.handle(request),
      })
      build.onComplete(({ manifest }) => bridge.validateBuild(manifest))
    },
  })
}
