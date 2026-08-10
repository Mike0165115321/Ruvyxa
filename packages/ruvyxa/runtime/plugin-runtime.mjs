import { existsSync, readFileSync } from 'node:fs'
import path from 'node:path'
import { createInterface } from 'node:readline'
import { fileURLToPath, pathToFileURL } from 'node:url'

import {
  cacheFileName,
  compileBundle,
  runtimeAliases,
  serverPlatform,
  toImportPath,
} from './compiler.mjs'

const [projectRootArg, mode] = process.argv.slice(2)

if (!projectRootArg || !mode) {
  writeResponse(failure('RUV1701', 'Plugin runtime requires project root and mode arguments.'))
  process.exit(1)
}

// Stdout is reserved for the NDJSON protocol.
console.log = console.info = console.debug = (...args) => console.error(...args)

const projectRoot = path.resolve(projectRootArg)
const runtimeDir = path.dirname(fileURLToPath(import.meta.url))

try {
  const registry = await loadRegistry(projectRoot)
  if (mode === '--persistent') {
    await runPersistent(registry)
  } else {
    const payload = JSON.parse(readFileSync(0, 'utf8'))
    const response = await handleHook(registry, mode, payload)
    writeResponse(response)
    if (!response.ok) process.exitCode = 1
  }
} catch (error) {
  writeResponse(failureFromError(error), mode === '--persistent')
  process.exitCode = 1
}

async function loadRegistry(root) {
  const configFile = findConfig(root)
  if (!configFile) return createRegistry(root, [])

  const moduleCode = `export { default } from ${JSON.stringify(toImportPath(configFile))}`
  const outfile = path.join(
    root,
    '.ruvyxa',
    'cache',
    'config',
    cacheFileName([moduleCode, configFile, 'plugin-runtime'], 'mjs'),
  )
  await compileBundle({
    projectRoot: root,
    entrySource: moduleCode,
    sourcefile: 'ruvyxa:plugin-config-entry.ts',
    outfile,
    platform: serverPlatform(),
    bundleAliasDependencies: true,
    aliases: runtimeAliases(runtimeDir),
  })

  const mod = await import(pathToFileURL(outfile).href + `?t=${Date.now()}`)
  const config = mod.default ?? {}
  const configuredPlugins = Array.isArray(config.plugins) ? config.plugins : []
  const contentPlugin = await configuredContentPlugin(root, configFile, config)
  return createRegistry(
    root,
    contentPlugin ? [...configuredPlugins, contentPlugin] : configuredPlugins,
  )
}

async function configuredContentPlugin(root, configFile, config) {
  const content = config?.content
  const enabled =
    content === true ||
    (content &&
      typeof content === 'object' &&
      !Array.isArray(content) &&
      (content.engine === true ||
        (content.engine && typeof content.engine === 'object' && !Array.isArray(content.engine))))
  if (!enabled) return undefined

  const moduleCode = 'export { contentEngineFromConfig as default } from "ruvyxa/plugins"'
  const outfile = path.join(
    root,
    '.ruvyxa',
    'cache',
    'config',
    cacheFileName([moduleCode, configFile, 'content-engine-runtime'], 'mjs'),
  )
  await compileBundle({
    projectRoot: root,
    entrySource: moduleCode,
    sourcefile: 'ruvyxa:content-engine-config-entry.ts',
    outfile,
    platform: serverPlatform(),
    bundleAliasDependencies: true,
    aliases: runtimeAliases(runtimeDir),
  })
  const mod = await import(pathToFileURL(outfile).href + `?t=${Date.now()}`)
  return mod.default(config)
}

function findConfig(root) {
  for (const fileName of [
    'ruvyxa.config.ts',
    'ruvyxa.config.mts',
    'ruvyxa.config.js',
    'ruvyxa.config.mjs',
  ]) {
    const file = path.join(root, fileName)
    if (existsSync(file)) return file
  }
  return null
}

async function createRegistry(root, pluginsValue) {
  const plugins = Array.isArray(pluginsValue) ? pluginsValue : []
  const names = new Set()
  const routeOwners = new Map()
  const registry = {
    root,
    plugins: [],
    httpRequest: [],
    httpResponse: [],
    buildStart: [],
    buildResolve: [],
    buildLoad: [],
    buildTransform: [],
    buildComplete: [],
    devFileChange: [],
    diagnostics: [],
    capabilities: new Map(),
  }

  for (const [index, plugin] of plugins.entries()) {
    if (!plugin || typeof plugin !== 'object' || Array.isArray(plugin)) {
      throw new TypeError(`config.plugins[${index}] must be a plugin object`)
    }
    const name = typeof plugin.name === 'string' ? plugin.name.trim() : ''
    if (!name) throw new TypeError(`config.plugins[${index}] must have a non-empty name`)
    if (names.has(name)) throw new TypeError(`duplicate plugin name: ${name}`)
    if (typeof plugin.register !== 'function') {
      throw new TypeError(`plugin "${name}" must provide register(api)`)
    }
    names.add(name)
    registry.plugins.push(name)
    await plugin.register(createRegistrationApi(registry, name, routeOwners))
  }

  const errors = registry.diagnostics.filter((diagnostic) => diagnostic.level === 'error')
  if (errors.length > 0) {
    throw new TypeError(
      errors.map((diagnostic) => `${diagnostic.code} ${diagnostic.message}`).join('\n'),
    )
  }
  return registry
}

function createRegistrationApi(registry, plugin, routeOwners) {
  const api = {
    http: Object.freeze({
      onRequest(value) {
        const registration = normalizeHttpHook(plugin, 'onRequest', value)
        registry.httpRequest.push({ plugin, kind: 'hook', ...registration })
      },
      onResponse(value) {
        registry.httpResponse.push({
          plugin,
          ...normalizeHttpHook(plugin, 'onResponse', value),
        })
      },
      route(value) {
        const route = normalizeHttpRoute(plugin, value)
        for (const method of route.methods) {
          const key = `${method} ${route.path}`
          const wildcardKey = `* ${route.path}`
          const conflict = routeOwners.get(key) ?? routeOwners.get(wildcardKey)
          if (conflict) {
            throw new TypeError(
              `plugin "${plugin}" route ${key} conflicts with plugin "${conflict}"`,
            )
          }
          if (method === '*') {
            const pathConflict = [...routeOwners.entries()].find(([candidate]) =>
              candidate.endsWith(` ${route.path}`),
            )
            if (pathConflict) {
              throw new TypeError(
                `plugin "${plugin}" route ${key} conflicts with plugin "${pathConflict[1]}"`,
              )
            }
          }
          routeOwners.set(key, plugin)
        }
        registry.httpRequest.push({ plugin, kind: 'route', ...route })
      },
    }),
    build: Object.freeze({
      onStart(hook) {
        registerHook(registry.buildStart, plugin, 'build.onStart', hook)
      },
      onResolve(hook) {
        registerHook(registry.buildResolve, plugin, 'build.onResolve', hook)
      },
      onLoad(hook) {
        registerHook(registry.buildLoad, plugin, 'build.onLoad', hook)
      },
      onTransform(hook) {
        registerHook(registry.buildTransform, plugin, 'build.onTransform', hook)
      },
      onComplete(hook) {
        registerHook(registry.buildComplete, plugin, 'build.onComplete', hook)
      },
    }),
    dev: Object.freeze({
      onFileChange(value) {
        const registration = normalizeDevFileChange(plugin, value)
        registry.devFileChange.push({ plugin, ...registration })
      },
    }),
    diagnostics: Object.freeze({
      report(value) {
        registry.diagnostics.push(normalizeDiagnostic(plugin, value))
      },
    }),
    native: Object.freeze({
      claim(capability, options = {}) {
        if (capability !== 'realtime@1') {
          throw new TypeError(
            `plugin "${plugin}" requested unsupported native capability "${String(capability)}"`,
          )
        }
        const owner = registry.capabilities.get(capability)
        if (owner) {
          throw new TypeError(
            `plugin "${plugin}" cannot claim ${capability}; it is already owned by plugin "${owner.plugin}"`,
          )
        }
        registry.capabilities.set(capability, normalizeRealtime(plugin, options))
      },
    }),
  }
  return Object.freeze(api)
}

function registerHook(collection, plugin, socket, hook) {
  if (typeof hook !== 'function') {
    throw new TypeError(`plugin "${plugin}" ${socket}() expects a function`)
  }
  collection.push({ plugin, hook })
}

function normalizeHttpHook(plugin, socket, value) {
  const registration = typeof value === 'function' ? { handler: value } : value
  if (!registration || typeof registration !== 'object' || Array.isArray(registration)) {
    throw new TypeError(`plugin "${plugin}" http.${socket}() expects a handler or options object`)
  }
  if (typeof registration.handler !== 'function') {
    throw new TypeError(`plugin "${plugin}" http.${socket}() requires handler`)
  }
  return {
    match: normalizePatterns(plugin, `http.${socket}().match`, registration.match),
    handler: registration.handler,
  }
}

function normalizeHttpRoute(plugin, value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`plugin "${plugin}" http.route() expects an options object`)
  }
  if (typeof value.path !== 'string' || !isExactApplicationPath(value.path)) {
    throw new TypeError(`plugin "${plugin}" http.route().path must be an exact absolute path`)
  }
  if (typeof value.handler !== 'function') {
    throw new TypeError(`plugin "${plugin}" http.route() requires handler`)
  }
  const input =
    value.method === undefined ? ['*'] : Array.isArray(value.method) ? value.method : [value.method]
  if (
    input.length === 0 ||
    input.some(
      (method) =>
        typeof method !== 'string' || !/^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/.test(method.trim()),
    )
  ) {
    throw new TypeError(
      `plugin "${plugin}" http.route().method must contain valid HTTP method tokens`,
    )
  }
  return {
    path: value.path,
    methods: [...new Set(input.map((method) => method.trim().toUpperCase()))],
    handler: value.handler,
  }
}

function normalizeDevFileChange(plugin, value) {
  const registration = typeof value === 'function' ? { handler: value } : value
  if (!registration || typeof registration !== 'object' || Array.isArray(registration)) {
    throw new TypeError(`plugin "${plugin}" dev.onFileChange() expects a handler or options object`)
  }
  if (typeof registration.handler !== 'function') {
    throw new TypeError(`plugin "${plugin}" dev.onFileChange() requires handler`)
  }
  return {
    match: normalizePatterns(plugin, 'dev.onFileChange().match', registration.match, false),
    handler: registration.handler,
  }
}

function normalizePatterns(plugin, field, value, requireSlash = true) {
  if (value === undefined) return undefined
  if (!Array.isArray(value) || value.length === 0) {
    throw new TypeError(`plugin "${plugin}" ${field} must contain at least one pattern`)
  }
  if (value.some((pattern) => typeof pattern !== 'string')) {
    throw new TypeError(`plugin "${plugin}" ${field} must be an array of strings`)
  }
  for (const [index, pattern] of value.entries()) {
    const wildcard = pattern.indexOf('*')
    const validStart = !requireSlash || pattern === '*' || pattern.startsWith('/')
    const validWildcard =
      wildcard === -1 || (wildcard === pattern.length - 1 && wildcard === pattern.lastIndexOf('*'))
    if (!pattern || !validStart || !validWildcard) {
      throw new TypeError(
        `plugin "${plugin}" ${field}[${index}] must ${requireSlash ? 'start with "/" and ' : ''}use a wildcard only at the end`,
      )
    }
  }
  return [...value]
}

function isExactApplicationPath(value) {
  return (
    value.startsWith('/') && !value.includes('?') && !value.includes('#') && !value.includes('*')
  )
}

function normalizeDiagnostic(plugin, value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`plugin "${plugin}" diagnostics.report() expects an object`)
  }
  if (!['info', 'warning', 'error'].includes(value.level)) {
    throw new TypeError(`plugin "${plugin}" diagnostic level must be info, warning, or error`)
  }
  if (typeof value.code !== 'string' || !/^[A-Z][A-Z0-9_-]{2,31}$/.test(value.code)) {
    throw new TypeError(`plugin "${plugin}" diagnostic code must be an uppercase identifier`)
  }
  if (typeof value.message !== 'string' || !value.message.trim()) {
    throw new TypeError(`plugin "${plugin}" diagnostic message must be non-empty`)
  }
  return Object.freeze({
    plugin,
    level: value.level,
    code: value.code,
    message: value.message.trim(),
  })
}

function normalizeRealtime(plugin, value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`plugin "${plugin}" native.claim('realtime@1') expects an options object`)
  }
  const pathValue = value.path ?? '/__ruvyxa/realtime'
  const heartbeatMs = value.heartbeatMs ?? 25_000
  const capacity = value.capacity ?? 256
  if (!isExactApplicationPath(pathValue)) {
    throw new TypeError(`plugin "${plugin}" realtime path must be an exact absolute path`)
  }
  if (!Number.isInteger(heartbeatMs) || heartbeatMs < 5_000 || heartbeatMs > 120_000) {
    throw new TypeError(`plugin "${plugin}" realtime heartbeatMs must be between 5000 and 120000`)
  }
  if (!Number.isInteger(capacity) || capacity < 16 || capacity > 4096) {
    throw new TypeError(`plugin "${plugin}" realtime capacity must be between 16 and 4096`)
  }
  const reserved = ['/__ruvyxa/hmr', '/__ruvyxa/client', '/__ruvyxa/action', '/__ruvyxa/trace']
  if (reserved.includes(pathValue)) {
    throw new TypeError(
      `plugin "${plugin}" realtime path "${pathValue}" collides with a reserved framework route`,
    )
  }
  return Object.freeze({ id: 'realtime@1', plugin, path: pathValue, heartbeatMs, capacity })
}

async function runPersistent(registry) {
  const lines = createInterface({ input: process.stdin, crlfDelay: Infinity })
  for await (const line of lines) {
    if (!line.trim()) continue
    let response
    try {
      const payload = JSON.parse(line)
      response = await handleHook(registry, payload.hook, payload)
    } catch (error) {
      response = failureFromError(error)
    }
    writeResponse(response, true)
  }
}

async function handleHook(registry, hook, payload) {
  switch (hook) {
    case 'describe':
      return success(describeRegistry(registry))
    case 'http.request':
      return success(await runHttpRequest(registry, payload))
    case 'http.response':
      return success(await runHttpResponse(registry, payload))
    case 'build.start':
      await runBuildStart(registry, payload)
      return success(null)
    case 'build.resolve':
      return success(await runBuildResolve(registry, payload))
    case 'build.load':
      return success(await runBuildLoad(registry, payload))
    case 'build.transform':
      return success(await runBuildTransform(registry, payload))
    case 'build.complete':
      await runBuildComplete(registry, payload)
      return success(null)
    case 'dev.fileChange':
      await runDevFileChange(registry, payload)
      return success(null)
    default:
      return failure('RUV1701', `Unknown plugin hook: ${hook}`)
  }
}

function describeRegistry(registry) {
  return {
    plugins: registry.plugins,
    http: {
      request: registry.httpRequest.length,
      response: registry.httpResponse.length,
      routes: registry.httpRequest.filter((entry) => entry.kind === 'route').length,
      requestMatch: patternUnion(registry.httpRequest),
      responseMatch: patternUnion(registry.httpResponse),
    },
    build: {
      start: registry.buildStart.length,
      resolve: registry.buildResolve.length,
      load: registry.buildLoad.length,
      transform: registry.buildTransform.length,
      complete: registry.buildComplete.length,
    },
    dev: { fileChange: registry.devFileChange.length },
    diagnostics: registry.diagnostics,
    capabilities: [...registry.capabilities.values()],
  }
}

async function runBuildResolve(registry, payload) {
  const base = buildContext(registry, payload)
  const context = Object.freeze({
    ...base,
    id: String(payload.id ?? ''),
    importer: payload.importer ?? undefined,
  })
  for (const entry of registry.buildResolve) {
    const result = await entry.hook(context)
    if (typeof result === 'string') return result
    if (result !== null && result !== undefined)
      throw unsupportedReturn(entry.plugin, 'build.onResolve')
  }
  return null
}

async function runBuildLoad(registry, payload) {
  const context = Object.freeze({
    ...buildContext(registry, payload),
    id: String(payload.id ?? ''),
  })
  for (const entry of registry.buildLoad) {
    const result = await entry.hook(context)
    const normalized = normalizeCodeResult(entry.plugin, 'build.onLoad', result)
    if (normalized) return normalized
  }
  return null
}

async function runBuildTransform(registry, payload) {
  let code = String(payload.code ?? '')
  let map
  let changed = false
  const base = buildContext(registry, payload)
  for (const entry of registry.buildTransform) {
    const context = Object.freeze({ ...base, code, id: String(payload.id ?? '') })
    const result = normalizeCodeResult(entry.plugin, 'build.onTransform', await entry.hook(context))
    if (!result) continue
    code = result.code
    if (result.map !== undefined) map = result.map
    changed = true
  }
  return changed ? { code, ...(map === undefined ? {} : { map }) } : null
}

function normalizeCodeResult(plugin, socket, result) {
  if (result === null || result === undefined) return null
  if (typeof result === 'string') return { code: result }
  if (result && typeof result === 'object' && typeof result.code === 'string') {
    return {
      code: result.code,
      ...(result.map === undefined || result.map === null
        ? {}
        : { map: typeof result.map === 'string' ? result.map : JSON.stringify(result.map) }),
    }
  }
  throw unsupportedReturn(plugin, socket)
}

function buildContext(registry, payload) {
  const allowed = new Set(['client', 'server', 'edge', 'worker', 'shared'])
  const environment = allowed.has(payload.environment) ? payload.environment : 'client'
  return { root: registry.root, environment }
}

async function runBuildStart(registry, payload) {
  const context = Object.freeze({ root: registry.root, outDir: path.resolve(payload.outDir) })
  for (const entry of registry.buildStart) await entry.hook(context)
}

async function runBuildComplete(registry, payload) {
  const context = Object.freeze({
    root: registry.root,
    outDir: path.resolve(payload.outDir),
    manifest: Object.freeze(payload.manifest ?? {}),
  })
  for (const entry of registry.buildComplete) await entry.hook(context)
}

async function runHttpRequest(registry, payload) {
  let request = requestFromPayload(payload.request)
  for (const entry of registry.httpRequest) {
    const pathname = decodedRequestPathname(request)
    if (entry.kind === 'route') {
      if (
        entry.path !== pathname ||
        (!entry.methods.includes('*') && !entry.methods.includes(request.method))
      )
        continue
      const result = await entry.handler(
        Object.freeze({ plugin: entry.plugin, root: registry.root, request: request.clone() }),
      )
      if (!(result instanceof Response)) throw unsupportedReturn(entry.plugin, 'http.route')
      return { kind: 'response', response: await responseToPayload(result) }
    }
    if (!matchesPatterns(entry.match, pathname)) continue
    let continued = request
    const context = Object.freeze({
      plugin: entry.plugin,
      root: registry.root,
      request: request.clone(),
      next(value = request) {
        if (!(value instanceof Request))
          throw new TypeError(`plugin "${entry.plugin}" http.onRequest().next() expects a Request`)
        continued = value
      },
    })
    const result = await entry.handler(context)
    if (result instanceof Response)
      return { kind: 'response', response: await responseToPayload(result) }
    if (result instanceof Request) request = result
    else if (result === undefined) request = continued
    else throw unsupportedReturn(entry.plugin, 'http.onRequest')
  }
  return { kind: 'request', request: await requestToPayload(request) }
}

async function runHttpResponse(registry, payload) {
  const request = requestFromPayload(payload.request)
  let response = responseFromPayload(payload.response)
  for (const entry of registry.httpResponse) {
    if (!matchesPatterns(entry.match, decodedRequestPathname(request))) continue
    let continued = response
    const context = Object.freeze({
      plugin: entry.plugin,
      root: registry.root,
      request: request.clone(),
      response: response.clone(),
      next(value = response) {
        if (!(value instanceof Response))
          throw new TypeError(
            `plugin "${entry.plugin}" http.onResponse().next() expects a Response`,
          )
        continued = value
      },
    })
    const result = await entry.handler(context)
    if (result instanceof Response) response = result
    else if (result === undefined) response = continued
    else throw unsupportedReturn(entry.plugin, 'http.onResponse')
  }
  return { response: await responseToPayload(response) }
}

async function runDevFileChange(registry, payload) {
  const paths = Array.isArray(payload.paths) ? payload.paths.map(String) : []
  for (const entry of registry.devFileChange) {
    const selected = entry.match
      ? paths.filter((value) => matchesPatterns(entry.match, value))
      : paths
    if (selected.length === 0) continue
    await entry.handler(Object.freeze({ root: registry.root, paths: Object.freeze(selected) }))
  }
}

function unsupportedReturn(plugin, socket) {
  return new TypeError(`plugin "${plugin}" ${socket} returned an unsupported value`)
}

function patternUnion(entries) {
  const patterns = new Set()
  for (const entry of entries) {
    if (entry.kind === 'route') {
      patterns.add(entry.path)
      continue
    }
    if (!entry.match || entry.match.length === 0 || entry.match.includes('*')) return null
    for (const pattern of entry.match) patterns.add(pattern)
  }
  return [...patterns]
}

function matchesPatterns(patterns, value) {
  if (!patterns || patterns.length === 0) return true
  return patterns.some((pattern) => {
    if (pattern === '*') return true
    if (pattern.endsWith('*')) return value.startsWith(pattern.slice(0, -1))
    return value === pattern
  })
}

/** Match paths using the decoded representation the Rust development router exposes to plugins. */
function decodedRequestPathname(request) {
  const pathname = new URL(request.url).pathname
  try {
    return decodeURIComponent(pathname)
  } catch {
    // A production host rejects malformed path encodings before this runtime
    // receives them. Preserve the encoded value defensively for direct calls.
    return pathname
  }
}

function requestFromPayload(value = {}) {
  const pathname = typeof value.path === 'string' && value.path.startsWith('/') ? value.path : '/'
  const method = String(value.method ?? 'GET').toUpperCase()
  const body = method === 'GET' || method === 'HEAD' ? undefined : decodeBody(value.bodyBase64)
  return new Request(`http://ruvyxa.local${pathname}`, {
    method,
    headers: headersFromPairs(value.headers),
    body,
  })
}

function responseFromPayload(value = {}) {
  return new Response(decodeBody(value.bodyBase64), {
    status: Number(value.status ?? 200),
    headers: headersFromPairs(value.headers),
  })
}

async function requestToPayload(request) {
  const url = new URL(request.url)
  return {
    method: request.method,
    path: url.pathname + url.search,
    headers: headerPairs(request.headers),
    bodyBase64: await encodeBody(request),
  }
}

async function responseToPayload(response) {
  return {
    status: response.status,
    headers: headerPairs(response.headers),
    bodyBase64: await encodeBody(response),
  }
}

function headersFromPairs(value) {
  const headers = new Headers()
  if (Array.isArray(value)) {
    for (const pair of value) {
      if (Array.isArray(pair) && pair.length === 2) headers.append(String(pair[0]), String(pair[1]))
    }
  }
  return headers
}

function headerPairs(headers) {
  const pairs = Array.from(headers.entries()).filter(([name]) => name !== 'set-cookie')
  const cookies = typeof headers.getSetCookie === 'function' ? headers.getSetCookie() : []
  for (const cookie of cookies) pairs.push(['set-cookie', cookie])
  return pairs
}

function decodeBody(value) {
  return typeof value === 'string' ? Buffer.from(value, 'base64') : undefined
}

async function encodeBody(message) {
  const bytes = Buffer.from(await message.arrayBuffer())
  return bytes.length > 0 ? bytes.toString('base64') : undefined
}

function success(result) {
  return { ok: true, result }
}

function failure(code, message, stack) {
  return { ok: false, code, message, stack }
}

function failureFromError(error) {
  return failure(
    error?.pluginCode === 'RUV1701' ? 'RUV1701' : 'RUV1700',
    error instanceof Error ? error.message : String(error),
    error?.stack,
  )
}

function writeResponse(response, newline = false) {
  process.stdout.write(JSON.stringify(response) + (newline ? '\n' : ''))
}
