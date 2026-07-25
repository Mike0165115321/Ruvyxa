import type { Adapter, AdapterArtifact, AdapterOutput, BuildContext } from '@ruvyxa/core'
import { clientBuildOutput, standaloneServerSource, validateBuildContext } from '@ruvyxa/core'

/** Options for Railway deployments. */
export interface RailwayAdapterOptions {
  /**
   * Emit a project-root `railway.json` using Railpack and the generated
   * standalone server. Existing configuration is never overwritten.
   * @default true
   */
  projectConfig?: boolean
}

/** Create a zero-config Railway deployment adapter for Ruvyxa. */
export function railwayAdapter(options: RailwayAdapterOptions = {}): Adapter {
  return {
    name: 'railway',
    target: 'node',
    supports: ['ssr', 'ssg', 'csr', 'isr', 'ppr', 'api'],
    build(ctx: BuildContext): AdapterOutput {
      validateBuildContext(ctx, 'railwayAdapter')

      const railwayConfig = JSON.stringify(
        {
          $schema: 'https://railway.com/railway.schema.json',
          build: {
            builder: 'RAILPACK',
            buildCommand: 'npm run build',
          },
          deploy: {
            startCommand: 'node .ruvyxa/deploy/railway/server/index.mjs',
            restartPolicyType: 'ON_FAILURE',
            restartPolicyMaxRetries: 10,
          },
        },
        null,
        2,
      )

      return {
        name: 'railway',
        target: 'node',
        platform: 'railway',
        runtime: 'node',
        entry: `${ctx.outDir}/server/app`,
        assetsDir: `${ctx.outDir}/assets`,
        ...clientBuildOutput(ctx),
        configFiles: ['railway.json'],
        artifacts: [
          {
            kind: 'function',
            path: 'deploy/railway/server',
            handlerSource: standaloneServerSource(),
          },
          { kind: 'static-site', path: 'deploy/railway/public', optional: true },
          {
            kind: 'file',
            path: 'deploy/railway/railway.json',
            contents: railwayConfig + '\n',
          },
          {
            kind: 'file',
            path: 'deploy/railway/README.md',
            contents:
              '# Ruvyxa on Railway\n\n' +
              'Railway auto-detects this adapter through `RAILWAY_PROJECT_ID`.\n' +
              'The generated server honors `PORT` and binds to `0.0.0.0`.\n\n' +
              '```bash\nnode .ruvyxa/deploy/railway/server/index.mjs\n```\n',
          },
          ...(options.projectConfig === false
            ? []
            : [
                {
                  kind: 'file',
                  path: 'railway.json',
                  scope: 'project',
                  skipIfExists: true,
                  contents: railwayConfig + '\n',
                } satisfies AdapterArtifact,
              ]),
        ],
      }
    },
  }
}

export default railwayAdapter
