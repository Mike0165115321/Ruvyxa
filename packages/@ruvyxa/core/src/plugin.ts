import type {
  PluginBuildDefinition,
  PluginDevDefinition,
  PluginDiagnostic,
  PluginHttpDefinition,
  PluginNativeDefinition,
  PluginRegistrationApi,
  RuvyxaPlugin,
  RuvyxaPluginDefinition,
} from './types.js'

export type {
  PluginBuildCompleteHook,
  PluginBuildContext,
  PluginBuildDefinition,
  PluginBuildLoadContext,
  PluginBuildLoadHandler,
  PluginBuildResolveContext,
  PluginBuildResolveHandler,
  PluginBuildSocket,
  PluginBuildStartContext,
  PluginBuildStartHook,
  PluginBuildTransformContext,
  PluginBuildTransformHandler,
  PluginDevFileChangeContext,
  PluginDevFileChangeHandler,
  PluginDevFileChangeRegistration,
  PluginDevDefinition,
  PluginDevSocket,
  PluginDiagnostic,
  PluginDiagnosticLevel,
  PluginDiagnosticsSocket,
  PluginEnvironment,
  PluginHttpContext,
  PluginHttpDefinition,
  PluginHttpRequestContext,
  PluginHttpRequestHandler,
  PluginHttpRequestRegistration,
  PluginHttpResponseContext,
  PluginHttpResponseHandler,
  PluginHttpResponseRegistration,
  PluginHttpRouteContext,
  PluginHttpRouteRegistration,
  PluginHttpSocket,
  PluginNativeCapability,
  PluginNativeDefinition,
  PluginNativeSocket,
  PluginRegistrationApi,
  PluginRoutePattern,
  PluginTransformContext,
  RealtimePluginOptions,
  RuvyxaPlugin,
  RuvyxaPluginDefinition,
  TransformResult,
} from './types.js'

/** Define a plugin through concise declarations or the advanced socket API. */
export function definePlugin(definition: RuvyxaPluginDefinition): RuvyxaPlugin {
  if (!definition || typeof definition !== 'object') {
    throw new TypeError('Ruvyxa plugin must be an object.')
  }
  if (typeof definition.name !== 'string' || definition.name.trim() === '') {
    throw new TypeError('Ruvyxa plugin must have a non-empty name.')
  }
  if (definition.register !== undefined && typeof definition.register !== 'function') {
    throw new TypeError(`Ruvyxa plugin "${definition.name}" register must be a function.`)
  }

  const headers = normalizeHeaders(definition.headers)
  const http = normalizeHttp(definition.http, definition.name)
  const build = normalizeBuild(definition.build, definition.name)
  const dev = normalizeDev(definition.dev, definition.name)
  const diagnostics = normalizeDiagnostics(definition.diagnostics)
  const native = normalizeNative(definition.native, definition.name)
  if (!definition.register && !headers && !http && !build && !dev && !diagnostics && !native) {
    throw new TypeError(
      `Ruvyxa plugin "${definition.name}" must declare behavior or provide register(api).`,
    )
  }

  return Object.freeze({
    name: definition.name.trim(),
    register(api: PluginRegistrationApi) {
      registerHttp(api, http, headers)
      registerBuild(api, build)
      if (dev?.onFileChange) api.dev.onFileChange(dev.onFileChange)
      for (const diagnostic of diagnostics ?? []) api.diagnostics.report(diagnostic)
      if (native?.realtime) {
        api.native.claim('realtime@1', native.realtime === true ? {} : native.realtime)
      }
      return definition.register?.(api)
    },
  })
}

function registerHttp(
  api: PluginRegistrationApi,
  http: PluginHttpDefinition | undefined,
  headers: readonly [string, string][] | undefined,
): void {
  if (http?.onRequest) api.http.onRequest({ match: http.match, handler: http.onRequest })
  if (http?.onResponse) api.http.onResponse({ match: http.match, handler: http.onResponse })
  for (const route of http?.routes ?? []) api.http.route(route)
  if (headers) {
    api.http.onResponse({
      match: http?.match,
      handler({ response }) {
        return withResponseHeaders(response, headers)
      },
    })
  }
}

function registerBuild(api: PluginRegistrationApi, build: PluginBuildDefinition | undefined): void {
  if (!build) return
  if (build.onStart) api.build.onStart(build.onStart)
  if (build.onResolve) api.build.onResolve(build.onResolve)
  if (build.onLoad) api.build.onLoad(build.onLoad)
  if (build.onTransform) api.build.onTransform(build.onTransform)
  if (build.onComplete) api.build.onComplete(build.onComplete)
}

function normalizeHeaders(
  headers: HeadersInit | undefined,
): readonly [string, string][] | undefined {
  if (headers === undefined) return undefined
  const entries: [string, string][] = []
  new Headers(headers).forEach((value, name) => entries.push([name, value]))
  return entries
}

function normalizeHttp(
  http: PluginHttpDefinition | undefined,
  pluginName: string,
): PluginHttpDefinition | undefined {
  if (http === undefined) return undefined
  if (!http || typeof http !== 'object' || Array.isArray(http)) {
    throw new TypeError(`Ruvyxa plugin "${pluginName}" http must be an object.`)
  }
  if (http.onRequest !== undefined && typeof http.onRequest !== 'function') {
    throw new TypeError(`Ruvyxa plugin "${pluginName}" http.onRequest must be a function.`)
  }
  if (http.onResponse !== undefined && typeof http.onResponse !== 'function') {
    throw new TypeError(`Ruvyxa plugin "${pluginName}" http.onResponse must be a function.`)
  }
  if (http.routes !== undefined && !Array.isArray(http.routes)) {
    throw new TypeError(`Ruvyxa plugin "${pluginName}" http.routes must be an array.`)
  }
  if (!http.onRequest && !http.onResponse && !http.routes) {
    throw new TypeError(`Ruvyxa plugin "${pluginName}" http must declare behavior.`)
  }
  return http
}

function normalizeBuild(
  build: PluginBuildDefinition | undefined,
  pluginName: string,
): PluginBuildDefinition | undefined {
  if (build === undefined) return undefined
  if (!build || typeof build !== 'object' || Array.isArray(build)) {
    throw new TypeError(`Ruvyxa plugin "${pluginName}" build must be an object.`)
  }
  const entries = Object.entries(build)
  if (entries.length === 0) {
    throw new TypeError(`Ruvyxa plugin "${pluginName}" build must declare behavior.`)
  }
  for (const [name, hook] of entries) {
    if (typeof hook !== 'function') {
      throw new TypeError(`Ruvyxa plugin "${pluginName}" build.${name} must be a function.`)
    }
  }
  return build
}

function normalizeDev(
  dev: PluginDevDefinition | undefined,
  pluginName: string,
): PluginDevDefinition | undefined {
  if (dev === undefined) return undefined
  if (!dev || typeof dev !== 'object' || Array.isArray(dev)) {
    throw new TypeError(`Ruvyxa plugin "${pluginName}" dev must be an object.`)
  }
  if (!dev.onFileChange) {
    throw new TypeError(`Ruvyxa plugin "${pluginName}" dev must declare onFileChange.`)
  }
  return dev
}

function normalizeDiagnostics(
  diagnostics: RuvyxaPluginDefinition['diagnostics'],
): readonly PluginDiagnostic[] | undefined {
  if (diagnostics === undefined) return undefined
  return Array.isArray(diagnostics)
    ? (diagnostics as readonly PluginDiagnostic[])
    : [diagnostics as PluginDiagnostic]
}

function normalizeNative(
  native: PluginNativeDefinition | undefined,
  pluginName: string,
): PluginNativeDefinition | undefined {
  if (native === undefined) return undefined
  if (!native || typeof native !== 'object' || Array.isArray(native)) {
    throw new TypeError(`Ruvyxa plugin "${pluginName}" native must be an object.`)
  }
  if (!native.realtime) {
    throw new TypeError(`Ruvyxa plugin "${pluginName}" native must declare realtime.`)
  }
  return native
}

/** Return a response copy with one header replaced, preserving status and body. */
export function withResponseHeader(response: Response, name: string, value: string): Response {
  return withResponseHeaders(response, [[name, value]])
}

function withResponseHeaders(response: Response, entries: readonly [string, string][]): Response {
  const headers = new Headers(response.headers)
  for (const [name, value] of entries) headers.set(name, value)
  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers,
  })
}
