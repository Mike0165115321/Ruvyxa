import { existsSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

import {
  cacheFileName,
  compileBundleWithMetadata,
  runtimeAliases,
  serverPlatform,
  toImportPath,
} from './compiler.mjs'

const [projectRootArg] = process.argv.slice(2)

if (!projectRootArg) {
  fail('RUV1601', 'Config renderer requires a project root argument.')
}

const projectRoot = path.resolve(projectRootArg)
const runtimeDir = path.dirname(fileURLToPath(import.meta.url))
const SITEMAP_CHANGE_FREQUENCIES = new Set([
  'always',
  'hourly',
  'daily',
  'weekly',
  'monthly',
  'yearly',
  'never',
])

try {
  const configFile = findConfig(projectRoot)
  if (!configFile) {
    await ok({}, 'no-config')
  }

  const moduleCode = `export { default } from ${JSON.stringify(toImportPath(configFile))}`
  const outfile = path.join(
    projectRoot,
    '.ruvyxa',
    'cache',
    'config',
    cacheFileName([moduleCode, configFile], 'mjs'),
  )

  const bundle = await compileBundleWithMetadata({
    projectRoot,
    entrySource: moduleCode,
    sourcefile: 'ruvyxa:config-entry.ts',
    outfile,
    platform: serverPlatform(),
    bundleAliasDependencies: true,
    aliases: runtimeAliases(runtimeDir),
  })

  const mod = await import(pathToFileURL(outfile).href + `?t=${Date.now()}`)
  const config = mod.default ?? {}
  await ok(await sanitizeConfig(config), bundle.dependencyHash)
} catch (error) {
  await fail('RUV1600', error instanceof Error ? error.message : String(error), error?.stack)
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

async function sanitizeConfig(config) {
  assertKnownKeys(config, 'config', [
    'appDir',
    'outDir',
    'runtime',
    'react',
    'typescript',
    'css',
    'server',
    'build',
    'render',
    'debug',
    'image',
    'security',
    'cache',
    'site',
    'middleware',
    'adapter',
    'adapterOptions',
    'plugins',
  ])
  assertKnownKeys(config.css, 'config.css', ['entries'])
  assertKnownKeys(config.server, 'config.server', ['host', 'port'])
  assertKnownKeys(config.build, 'config.build', [
    'minify',
    'map',
    'treeShake',
    'split',
    'workers',
    'jsx',
    'target',
    'manifest',
    'warm',
    'prerenderCache',
  ])
  assertKnownKeys(config.debug, 'config.debug', ['overlay', 'traces'])
  assertKnownKeys(config.image, 'config.image', [
    'optimize',
    'quality',
    'lossless',
    'keepOriginal',
    'variantWidths',
    'workers',
  ])
  assertKnownKeys(config.security, 'config.security', [
    'actionLimit',
    'apiLimit',
    'pluginLimit',
    'actionRateLimit',
    'sameOrigin',
    'fetchMeta',
    'trustedProxyIps',
    'headers',
  ])
  assertKnownKeys(config.security?.actionRateLimit, 'config.security.actionRateLimit', [
    'max',
    'window',
  ])
  assertKnownKeys(config.cache, 'config.cache', ['routes', 'css', 'dir'])
  assertKnownKeys(config.site, 'config.site', ['url', 'sitemap', 'robots'])
  assertKnownKeys(config.site?.sitemap, 'config.site.sitemap', [
    'exclude',
    'additionalPaths',
    'defaults',
    'entries',
  ])
  assertKnownKeys(config.site?.sitemap?.defaults, 'config.site.sitemap.defaults', [
    'lastModified',
    'changeFrequency',
    'priority',
  ])
  for (const [index, entry] of sitemapEntries(config.site?.sitemap?.entries).entries()) {
    const field = `config.site.sitemap.entries[${index}]`
    assertKnownKeys(entry, field, [
      'url',
      'lastModified',
      'changeFrequency',
      'priority',
      'alternates',
      'images',
      'videos',
    ])
    assertKnownKeys(entry?.alternates, `${field}.alternates`, ['languages'])
    for (const [videoIndex, video] of sitemapVideos(entry?.videos).entries()) {
      const videoField = `${field}.videos[${videoIndex}]`
      assertKnownKeys(video, videoField, [
        'title',
        'thumbnail_loc',
        'description',
        'content_loc',
        'player_loc',
        'duration',
        'view_count',
        'rating',
        'expiration_date',
        'publication_date',
        'family_friendly',
        'requires_subscription',
        'live',
        'restriction',
        'platform',
        'uploader',
        'tag',
      ])
      assertKnownKeys(video?.restriction, `${videoField}.restriction`, ['relationship', 'content'])
      assertKnownKeys(video?.platform, `${videoField}.platform`, ['relationship', 'content'])
      assertKnownKeys(video?.uploader, `${videoField}.uploader`, ['content', 'info'])
    }
  }
  assertKnownKeys(config.site?.robots, 'config.site.robots', ['rules', 'sitemap', 'host'])
  for (const [index, rule] of siteRobotsRules(config.site?.robots?.rules).entries()) {
    assertKnownKeys(rule, `config.site.robots.rules[${index}]`, [
      'userAgent',
      'allow',
      'disallow',
      'crawlDelay',
    ])
  }
  assertKnownKeys(config.render, 'config.render', ['strategy', 'revalidate'])
  assertKnownKeys(config.middleware, 'config.middleware', ['builtin', 'workers', 'timeoutMs'])
  assertKnownKeys(config.middleware?.builtin, 'config.middleware.builtin', [
    'cors',
    'timing',
    'log',
    'rate',
    'headers',
  ])
  assertKnownKeys(config.middleware?.builtin?.cors, 'config.middleware.builtin.cors', [
    'origins',
    'methods',
    'headers',
    'credentials',
    'maxAge',
  ])
  assertKnownKeys(config.middleware?.builtin?.rate, 'config.middleware.builtin.rate', [
    'max',
    'window',
    'key',
  ])
  assertConfigValueShape(config)

  return {
    appDir: stringValue(config.appDir),
    outDir: stringValue(config.outDir),
    runtime: stringValue(config.runtime),
    react: booleanValue(config.react),
    css: objectValue(config.css, {
      entries: stringArrayValue(config.css?.entries),
    }),
    server: objectValue(config.server, {
      host: stringValue(config.server?.host),
      port: numberValue(config.server?.port),
    }),
    build: objectValue(config.build, {
      minify: booleanValue(config.build?.minify),
      map: booleanValue(config.build?.map),
      treeShake: booleanValue(config.build?.treeShake),
      split: stringValue(config.build?.split),
      workers: numberValue(config.build?.workers),
      jsx: stringValue(config.build?.jsx),
      target: stringValue(config.build?.target),
      manifest: booleanValue(config.build?.manifest),
      warm: booleanValue(config.build?.warm),
      prerenderCache: booleanValue(config.build?.prerenderCache),
    }),
    render: objectValue(config.render, {
      strategy: stringValue(config.render?.strategy),
      revalidate: numberValue(config.render?.revalidate),
    }),
    debug: objectValue(config.debug, {
      overlay: booleanValue(config.debug?.overlay),
      traces: booleanValue(config.debug?.traces),
    }),
    image: objectValue(config.image, {
      optimize: booleanValue(config.image?.optimize),
      quality: numberValue(config.image?.quality),
      lossless: booleanValue(config.image?.lossless),
      keepOriginal: booleanValue(config.image?.keepOriginal),
      variantWidths: numberArrayValue(config.image?.variantWidths),
      workers: numberValue(config.image?.workers),
    }),
    security: objectValue(config.security, {
      actionLimit: numberValue(config.security?.actionLimit),
      apiLimit: numberValue(config.security?.apiLimit),
      pluginLimit: numberValue(config.security?.pluginLimit),
      actionRateLimit: objectValue(config.security?.actionRateLimit, {
        max: numberValue(config.security?.actionRateLimit?.max),
        window: numberValue(config.security?.actionRateLimit?.window),
      }),
      sameOrigin: booleanValue(config.security?.sameOrigin),
      fetchMeta: booleanValue(config.security?.fetchMeta),
      trustedProxyIps: stringArrayValue(config.security?.trustedProxyIps),
      headers: booleanValue(config.security?.headers),
    }),
    cache: objectValue(config.cache, {
      routes: booleanValue(config.cache?.routes),
      css: booleanValue(config.cache?.css),
      dir: stringValue(config.cache?.dir),
    }),
    site: siteValue(config.site),
    middleware: safeJsonValue(config.middleware),
    adapter: await adapterOutput(config.adapter, projectRoot, config.outDir),
    adapterOptions: safeJsonValue(config.adapterOptions),
    plugins: pluginDescriptors(config.plugins),
  }
}

function assertConfigValueShape(config) {
  assertShape(config, 'config', {
    appDir: 'string',
    outDir: 'string',
    runtime: 'string',
    react: 'boolean',
    typescript: { strict: 'boolean' },
    css: { entries: 'string[]' },
    server: { host: 'string', port: 'number' },
    build: {
      minify: 'boolean',
      map: 'boolean',
      treeShake: 'boolean',
      split: 'string',
      workers: 'number',
      jsx: 'string',
      target: 'string',
      manifest: 'boolean',
      warm: 'boolean',
      prerenderCache: 'boolean',
    },
    render: { strategy: 'string', revalidate: 'number' },
    debug: { overlay: 'boolean', traces: 'boolean' },
    image: {
      optimize: 'boolean',
      quality: 'number',
      lossless: 'boolean',
      keepOriginal: 'boolean',
      variantWidths: 'number[]',
      workers: 'number',
    },
    security: {
      actionLimit: 'number',
      apiLimit: 'number',
      pluginLimit: 'number',
      actionRateLimit: { max: 'number', window: 'number' },
      sameOrigin: 'boolean',
      fetchMeta: 'boolean',
      trustedProxyIps: 'string[]',
      headers: 'boolean',
    },
    cache: { routes: 'boolean', css: 'boolean', dir: 'string' },
    middleware: { workers: 'number', timeoutMs: 'number' },
    adapter: 'object',
    adapterOptions: 'object',
    plugins: 'array',
  })
  assertSiteShape(config.site)
}

function assertSiteShape(site) {
  if (site === undefined) return
  if (!isObject(site)) throw new Error('RUV1602 config.site must be an object.')
  if (site.url !== undefined && typeof site.url !== 'string') {
    throw new Error('RUV1602 config.site.url must be string.')
  }
  if (site.sitemap !== undefined) {
    if (typeof site.sitemap !== 'boolean' && !isObject(site.sitemap)) {
      throw new Error('RUV1602 config.site.sitemap must be boolean or object.')
    }
    if (isObject(site.sitemap)) {
      assertStringArray(site.sitemap.exclude, 'config.site.sitemap.exclude')
      assertStringArray(site.sitemap.additionalPaths, 'config.site.sitemap.additionalPaths')
      if (site.sitemap.defaults !== undefined && !isObject(site.sitemap.defaults)) {
        throw new Error('RUV1602 config.site.sitemap.defaults must be an object.')
      }
      if (isObject(site.sitemap.defaults)) {
        assertSitemapEntryMetadata(site.sitemap.defaults, 'config.site.sitemap.defaults')
      }
      if (site.sitemap.entries !== undefined && !Array.isArray(site.sitemap.entries)) {
        throw new Error('RUV1602 config.site.sitemap.entries must be an array.')
      }
      for (const [index, entry] of sitemapEntries(site.sitemap.entries).entries()) {
        assertSitemapEntry(entry, `config.site.sitemap.entries[${index}]`)
      }
    }
  }
  if (site.robots !== undefined) {
    if (typeof site.robots !== 'boolean' && !isObject(site.robots)) {
      throw new Error('RUV1602 config.site.robots must be boolean or object.')
    }
    if (isObject(site.robots)) {
      assertStringOrArray(site.robots.sitemap, 'config.site.robots.sitemap')
      if (site.robots.host !== undefined && typeof site.robots.host !== 'string') {
        throw new Error('RUV1602 config.site.robots.host must be string.')
      }
      const rules = siteRobotsRules(site.robots.rules)
      if (
        site.robots.rules !== undefined &&
        !isObject(site.robots.rules) &&
        !Array.isArray(site.robots.rules)
      ) {
        throw new Error('RUV1602 config.site.robots.rules must be object or array.')
      }
      for (const [index, rule] of rules.entries()) {
        if (!isObject(rule)) {
          throw new Error(`RUV1602 config.site.robots.rules[${index}] must be an object.`)
        }
        assertStringOrArray(rule.userAgent, `config.site.robots.rules[${index}].userAgent`)
        assertStringOrArray(rule.allow, `config.site.robots.rules[${index}].allow`)
        assertStringOrArray(rule.disallow, `config.site.robots.rules[${index}].disallow`)
        if (
          rule.crawlDelay !== undefined &&
          (!Number.isSafeInteger(rule.crawlDelay) || rule.crawlDelay < 0)
        ) {
          throw new Error(
            `RUV1602 config.site.robots.rules[${index}].crawlDelay must be a non-negative safe integer.`,
          )
        }
      }
    }
  }
}

function assertStringArray(value, field) {
  if (value === undefined) return
  if (!Array.isArray(value) || !value.every((item) => typeof item === 'string')) {
    throw new Error(`RUV1602 ${field} must be string[].`)
  }
}

function assertStringOrArray(value, field) {
  if (value === undefined) return
  if (
    typeof value !== 'string' &&
    (!Array.isArray(value) || !value.every((item) => typeof item === 'string'))
  ) {
    throw new Error(`RUV1602 ${field} must be string or string[].`)
  }
}

function siteRobotsRules(value) {
  if (value === undefined) return []
  return Array.isArray(value) ? value : [value]
}

function sitemapEntries(value) {
  return Array.isArray(value) ? value : []
}

function sitemapVideos(value) {
  return Array.isArray(value) ? value : []
}

function assertSitemapEntry(entry, field) {
  if (!isObject(entry)) throw new Error(`RUV1602 ${field} must be an object.`)
  if (typeof entry.url !== 'string' || entry.url === '') {
    throw new Error(`RUV1602 ${field}.url must be a non-empty string.`)
  }
  assertSitemapEntryMetadata(entry, field)
  if (entry.alternates !== undefined && !isObject(entry.alternates)) {
    throw new Error(`RUV1602 ${field}.alternates must be an object.`)
  }
  if (entry.alternates?.languages !== undefined) {
    if (
      !isObject(entry.alternates.languages) ||
      !Object.values(entry.alternates.languages).every((value) => typeof value === 'string')
    ) {
      throw new Error(`RUV1602 ${field}.alternates.languages must be a string record.`)
    }
  }
  assertStringArray(entry.images, `${field}.images`)
  if (entry.videos !== undefined && !Array.isArray(entry.videos)) {
    throw new Error(`RUV1602 ${field}.videos must be an array.`)
  }
  for (const [index, video] of sitemapVideos(entry.videos).entries()) {
    assertSitemapVideo(video, `${field}.videos[${index}]`)
  }
}

function assertSitemapEntryMetadata(value, field) {
  assertDateValue(value.lastModified, `${field}.lastModified`)
  if (
    value.changeFrequency !== undefined &&
    !SITEMAP_CHANGE_FREQUENCIES.has(value.changeFrequency)
  ) {
    throw new Error(`RUV1602 ${field}.changeFrequency must be a supported sitemap frequency.`)
  }
  if (
    value.priority !== undefined &&
    (!Number.isFinite(value.priority) || value.priority < 0 || value.priority > 1)
  ) {
    throw new Error(`RUV1602 ${field}.priority must be between 0 and 1.`)
  }
}

function assertSitemapVideo(video, field) {
  if (!isObject(video)) throw new Error(`RUV1602 ${field} must be an object.`)
  for (const key of ['title', 'thumbnail_loc', 'description']) {
    if (typeof video[key] !== 'string' || video[key] === '') {
      throw new Error(`RUV1602 ${field}.${key} must be a non-empty string.`)
    }
  }
  for (const key of ['content_loc', 'player_loc']) {
    if (video[key] !== undefined && typeof video[key] !== 'string') {
      throw new Error(`RUV1602 ${field}.${key} must be a string.`)
    }
  }
  for (const key of ['duration', 'view_count', 'rating']) {
    if (video[key] !== undefined && !Number.isFinite(video[key])) {
      throw new Error(`RUV1602 ${field}.${key} must be a finite number.`)
    }
  }
  assertDateValue(video.expiration_date, `${field}.expiration_date`)
  assertDateValue(video.publication_date, `${field}.publication_date`)
  for (const key of ['family_friendly', 'requires_subscription', 'live']) {
    if (video[key] !== undefined && video[key] !== 'yes' && video[key] !== 'no') {
      throw new Error(`RUV1602 ${field}.${key} must be "yes" or "no".`)
    }
  }
  for (const key of ['restriction', 'platform']) {
    const relationship = video[key]
    if (relationship === undefined) continue
    if (
      !isObject(relationship) ||
      (relationship.relationship !== 'allow' && relationship.relationship !== 'deny') ||
      typeof relationship.content !== 'string' ||
      relationship.content === ''
    ) {
      throw new Error(
        `RUV1602 ${field}.${key} must contain relationship "allow" or "deny" and string content.`,
      )
    }
  }
  if (video.uploader !== undefined) {
    if (
      !isObject(video.uploader) ||
      typeof video.uploader.content !== 'string' ||
      video.uploader.content === '' ||
      (video.uploader.info !== undefined && typeof video.uploader.info !== 'string')
    ) {
      throw new Error(`RUV1602 ${field}.uploader must contain string content and optional info.`)
    }
  }
  assertStringOrArray(video.tag, `${field}.tag`)
}

function assertDateValue(value, field) {
  if (value === undefined) return
  if (value instanceof Date) {
    if (!Number.isFinite(value.getTime())) throw new Error(`RUV1602 ${field} must be a valid Date.`)
    return
  }
  if (typeof value !== 'string') {
    throw new Error(`RUV1602 ${field} must be a string or Date.`)
  }
}

function siteValue(site) {
  if (!isObject(site)) return undefined
  return objectValue(site, {
    url: stringValue(site.url),
    sitemap: siteSettingValue(site.sitemap),
    robots: siteSettingValue(site.robots),
  })
}

function siteSettingValue(value) {
  if (typeof value === 'boolean') return value
  return isObject(value) ? safeJsonValue(value) : undefined
}

function assertShape(value, field, shape) {
  if (!isObject(value)) {
    throw new Error(`RUV1602 ${field} must be an object.`)
  }
  for (const [key, expected] of Object.entries(shape)) {
    const candidate = value[key]
    if (candidate === undefined) continue
    const childField = `${field}.${key}`
    if (typeof expected === 'object') {
      assertShape(candidate, childField, expected)
      continue
    }
    const valid =
      (expected === 'string' && typeof candidate === 'string') ||
      (expected === 'number' && Number.isFinite(candidate)) ||
      (expected === 'boolean' && typeof candidate === 'boolean') ||
      (expected === 'object' && isObject(candidate)) ||
      (expected === 'array' && Array.isArray(candidate)) ||
      (expected === 'string[]' &&
        Array.isArray(candidate) &&
        candidate.every((item) => typeof item === 'string')) ||
      (expected === 'number[]' &&
        Array.isArray(candidate) &&
        candidate.every((item) => Number.isFinite(item)))
    if (!valid) {
      throw new Error(`RUV1602 ${childField} must be ${expected}.`)
    }
  }
}

function isObject(value) {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

async function adapterOutput(adapter, root, outDir) {
  if (adapter === undefined) return undefined
  if (!adapter || typeof adapter !== 'object' || typeof adapter.build !== 'function') {
    throw new Error('RUV1603 config.adapter must provide a build(context) function.')
  }

  const output = await adapter.build({ root, outDir: stringValue(outDir) ?? '.ruvyxa' })
  if (!output || typeof output !== 'object') {
    throw new Error('RUV1603 config.adapter.build(context) must return an adapter output object.')
  }
  if (typeof output.name !== 'string' || typeof output.target !== 'string') {
    throw new Error('RUV1603 adapter output must include string name and target fields.')
  }

  const serialized = safeJsonValue(output)
  if (serialized === undefined) {
    throw new Error('RUV1603 adapter output must be JSON-serializable.')
  }

  return serialized
}

function assertKnownKeys(value, field, allowedKeys) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return
  const allowed = new Set(allowedKeys)
  const unknown = Object.keys(value).filter((key) => !allowed.has(key))
  if (unknown.length > 0) {
    throw new Error(
      `RUV1602 unknown ${field} field${unknown.length === 1 ? '' : 's'}: ${unknown.join(', ')}`,
    )
  }
}

function objectValue(source, value) {
  if (!source || typeof source !== 'object') return undefined
  const filtered = Object.fromEntries(
    Object.entries(value).filter(([, item]) => item !== undefined),
  )
  return Object.keys(filtered).length > 0 ? filtered : undefined
}

function stringValue(value) {
  return typeof value === 'string' ? value : undefined
}

function numberValue(value) {
  return Number.isFinite(value) ? value : undefined
}

function numberArrayValue(value) {
  if (!Array.isArray(value) || !value.every((item) => Number.isFinite(item))) return undefined
  return value
}

function booleanValue(value) {
  return typeof value === 'boolean' ? value : undefined
}

function stringArrayValue(value) {
  if (!Array.isArray(value) || !value.every((item) => typeof item === 'string')) return undefined
  return value
}

function safeJsonValue(value) {
  if (value === undefined) return undefined
  try {
    JSON.stringify(value)
    return value
  } catch {
    return undefined
  }
}

function pluginDescriptors(value) {
  if (!Array.isArray(value)) return undefined
  const names = new Set()
  const plugins = value.map((plugin, index) => {
    if (!isObject(plugin)) {
      throw new Error(`RUV1602 config.plugins[${index}] must be an object.`)
    }
    if (typeof plugin.name !== 'string' || plugin.name.trim() === '') {
      throw new Error(`RUV1602 config.plugins[${index}].name must be a non-empty string.`)
    }
    if (typeof plugin.register !== 'function') {
      throw new Error(`RUV1602 plugin "${plugin.name}" must provide register(api).`)
    }
    const name = plugin.name.trim()
    if (names.has(name)) {
      throw new Error(`RUV1602 duplicate plugin name: ${name}`)
    }
    names.add(name)
    // Head entries are declared once and injected by the server on every
    // render, so they travel with the descriptor instead of through a
    // per-request hook. `definePlugin` has already validated their shape.
    const head = Array.isArray(plugin.head) ? plugin.head.filter(isObject) : []
    return head.length > 0 ? { name, head } : { name }
  })

  return plugins.length > 0 ? plugins : undefined
}

async function ok(config, dependencyHash) {
  await writeJson({ ok: true, config, dependencyHash })
  process.exit(0)
}

async function fail(code, message, stack) {
  await writeJson({ ok: false, code, message, stack })
  process.exit(1)
}

function writeJson(payload) {
  return new Promise((resolve, reject) => {
    process.stdout.write(JSON.stringify(payload), (error) => {
      if (error) {
        reject(error)
      } else {
        resolve()
      }
    })
  })
}
