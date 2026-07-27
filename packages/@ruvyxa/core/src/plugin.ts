import type { RuvyxaPlugin, RuvyxaPluginDefinition } from './types.js'

export type {
  PluginBuildCompleteHook,
  PluginBuildContext,
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
  PluginDevSocket,
  PluginDiagnostic,
  PluginDiagnosticLevel,
  PluginDiagnosticsSocket,
  PluginEnvironment,
  PluginHttpContext,
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
  PluginNativeSocket,
  PluginRegistrationApi,
  PluginRoutePattern,
  PluginTransformContext,
  RealtimePluginOptions,
  RuvyxaPlugin,
  RuvyxaPluginDefinition,
  TransformResult,
} from './types.js'

/** Define a plugin for the versioned Ruvyxa socket API. */
export function definePlugin(definition: RuvyxaPluginDefinition): RuvyxaPlugin {
  if (!definition || typeof definition !== 'object') {
    throw new TypeError('Ruvyxa plugin must be an object.')
  }
  if (typeof definition.name !== 'string' || definition.name.trim() === '') {
    throw new TypeError('Ruvyxa plugin must have a non-empty name.')
  }
  if (typeof definition.register !== 'function') {
    throw new TypeError(`Ruvyxa plugin "${definition.name}" must provide register(api).`)
  }
  return Object.freeze({
    name: definition.name.trim(),
    register: definition.register,
  })
}

/** Return a response copy with one header replaced, preserving status and body. */
export function withResponseHeader(response: Response, name: string, value: string): Response {
  const headers = new Headers(response.headers)
  headers.set(name, value)
  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers,
  })
}
