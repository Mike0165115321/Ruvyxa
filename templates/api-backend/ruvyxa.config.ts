import { config, type RuvyxaConfig } from 'ruvyxa/config'

const settings: RuvyxaConfig = {
  appDir: 'app',
  outDir: '.ruvyxa',
  server: {
    host: 'localhost',
    port: 3000,
  },
  build: {
    minify: true,
    map: false,
    treeShake: true,
    split: 'route',
    // `workers` is intentionally unset: the build sizes route bundling to the
    // machine's cores and free memory. Pinning a number here caps a 16-core
    // machine at 4 and asks a memory-limited CI container for more than it has.
  },
  cache: {
    routes: true,
    css: true,
  },
  debug: {
    overlay: true,
  },
  image: {
    optimize: true,
    quality: 82,
    lossless: false,
    workers: 0,
  },
}

export default config(settings)
