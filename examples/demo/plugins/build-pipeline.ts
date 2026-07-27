import { definePlugin } from 'ruvyxa/plugin'

/** Demonstrates module resolution, source transforms, and build lifecycle hooks. */
export default definePlugin({
  name: 'demo-build-pipeline',

  register({ build }) {
    build.onResolve(({ id, root: projectRoot }) => {
      if (id !== '~demo-plugin') return undefined
      const root = projectRoot.replaceAll('/', '\\')
      return `${root}\\plugins\\virtual-message.ts`
    })

    build.onTransform(({ code, id, environment }) => {
      const normalizedId = id.replaceAll('\\', '/')
      if (environment !== 'client' || !normalizedId.endsWith('/plugin-lab/plugin-marker.ts')) {
        return undefined
      }

      return code.replace("'original'", "'transformed-by-plugin'")
    })

    build.onComplete(({ manifest }) => {
      console.info(
        `[demo-build-pipeline] completed a build with resolve, transform, and lifecycle sockets (${JSON.stringify(manifest).length} manifest characters)`,
      )
    })
  },
})
