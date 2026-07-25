import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import { railwayAdapter } from '../../../packages/@ruvyxa/adapter-railway/src/index.ts'

describe('railwayAdapter', () => {
  it('produces a standalone full-stack deployment and safe Railway config', async () => {
    const adapter = railwayAdapter()
    const output = await adapter.build({ root: '.', outDir: '.ruvyxa' })

    assert.equal(output.platform, 'railway')
    assert.equal(output.target, 'node')
    assert.equal(output.runtime, 'node')
    assert.deepEqual(adapter.supports, ['ssr', 'ssg', 'csr', 'isr', 'ppr', 'api'])
    assert.deepEqual(
      output.artifacts?.map(({ kind, path, scope }) => ({ kind, path, scope })),
      [
        { kind: 'function', path: 'deploy/railway/server', scope: undefined },
        { kind: 'static-site', path: 'deploy/railway/public', scope: undefined },
        { kind: 'file', path: 'deploy/railway/railway.json', scope: undefined },
        { kind: 'file', path: 'deploy/railway/README.md', scope: undefined },
        { kind: 'file', path: 'railway.json', scope: 'project' },
      ],
    )

    const server = output.artifacts?.find((artifact) => artifact.kind === 'function')
    assert.match(String(server && 'handlerSource' in server ? server.handlerSource : ''), /PORT/)
    assert.match(
      String(server && 'handlerSource' in server ? server.handlerSource : ''),
      /0\.0\.0\.0/,
    )

    const configArtifact = output.artifacts?.find((artifact) => artifact.path === 'railway.json')
    assert.equal(configArtifact?.skipIfExists, true)
    const config = JSON.parse(
      configArtifact && 'contents' in configArtifact ? String(configArtifact.contents) : '{}',
    )
    assert.equal(config.build.builder, 'RAILPACK')
    assert.equal(config.build.buildCommand, 'npm run build')
    assert.equal(config.deploy.startCommand, 'node .ruvyxa/deploy/railway/server/index.mjs')
  })

  it('can avoid project-root configuration', async () => {
    const output = await railwayAdapter({ projectConfig: false }).build({
      root: '.',
      outDir: '.ruvyxa',
    })
    assert.equal(
      output.artifacts?.some((artifact) => artifact.scope === 'project'),
      false,
    )
  })
})
