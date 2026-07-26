import type {
  PluginMiddleware,
  PluginMiddlewareOptions,
  PluginRequestMiddleware,
  RuvyxaConfig,
  RuvyxaPlugin,
} from './types.js'

export type {
  BuiltinMiddlewareConfig,
  CachedStaticParams,
  CorsConfig,
  GetStaticParams,
  ImageConfig,
  MiddlewareConfig,
  PageProps,
  PluginBuildCompleteHook,
  PluginBuildContext,
  PluginEnvironment,
  PluginMiddleware,
  PluginMiddlewareOptions,
  PluginMiddlewareContext,
  PluginRequestMiddleware,
  PluginRequestResult,
  PluginResolveIdHook,
  PluginResponseMiddleware,
  PluginSetupContext,
  PluginTransformContext,
  PluginTransformHook,
  RateLimitConfig,
  RealtimePluginOptions,
  RenderConfig,
  RenderStrategy,
  RouteParamValue,
  RouteParams,
  RuvyxaConfig,
  RuvyxaPlugin,
  StaticParamsContext,
  StaticParamSegment,
  StaticParamsCacheDuration,
  StaticParamsResult,
  StaticParamsValues,
  TransformResult,
} from './types.js'

/** Define the typed contents of `ruvyxa.config.ts`. */
export function config<TConfig extends RuvyxaConfig>(config: TConfig): TConfig {
  return config
}

/** Define a named plugin for use in `ruvyxa.config.ts`. */
export function definePlugin(plugin: RuvyxaPlugin): RuvyxaPlugin {
  if (!plugin || typeof plugin !== 'object') {
    throw new TypeError('Ruvyxa plugin must be an object.')
  }
  if (typeof plugin.name !== 'string' || plugin.name.trim() === '') {
    throw new TypeError('Ruvyxa plugin must have a non-empty name.')
  }
  if (typeof plugin.setup !== 'function') {
    throw new TypeError(`Ruvyxa plugin "${plugin.name}" must provide setup(context).`)
  }
  return Object.freeze({ ...plugin, name: plugin.name.trim() })
}

/** Define a request/response middleware plugin without a setup wrapper. */
export function plugin(
  name: string,
  middleware: PluginMiddlewareOptions | PluginRequestMiddleware,
): RuvyxaPlugin {
  const normalizedMiddleware = normalizePluginMiddleware(middleware)
  return definePlugin({
    name,
    setup({ addMiddleware }) {
      addMiddleware(normalizedMiddleware)
    },
  })
}

/** Return a response copy with one header replaced, preserving its status and body. */
export function withResponseHeader(response: Response, name: string, value: string): Response {
  return withResponseHeaders(response, new Headers([[name, value]]))
}

function normalizePluginMiddleware(
  middleware: PluginMiddlewareOptions | PluginRequestMiddleware,
): PluginMiddleware | PluginRequestMiddleware {
  if (typeof middleware === 'function' || middleware.headers === undefined) return middleware

  const { headers: configuredHeaders, onResponse, ...rest } = middleware
  const responseHeaders = new Headers(configuredHeaders)
  return {
    ...rest,
    async onResponse(request, response, context) {
      const output = await onResponse?.(request, response, context)
      return withResponseHeaders(output ?? response, responseHeaders)
    },
  }
}

function withResponseHeaders(response: Response, additions: Headers): Response {
  const headers = new Headers(response.headers)
  additions.forEach((value, name) => headers.set(name, value))
  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers,
  })
}
