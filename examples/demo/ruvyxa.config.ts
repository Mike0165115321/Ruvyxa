import { config, type RuvyxaConfig } from 'ruvyxa/config'
import { realtime } from '@ruvyxa/realtime/plugin'
import { demoPlugins } from './plugins'

const settings: RuvyxaConfig = {
  appDir: 'app',
  outDir: '.ruvyxa',

  server: {
    host: 'localhost',
    port: 3000,
  },

  // robots.txt and sitemap.xml are generated from the route manifest at build.
  site: {
    url: 'https://demo.ruvyxa.dev',
  },

  build: {
    minify: true,
    map: false,
    treeShake: true,
    split: 'route',
    workers: 4,
  },

  render: {
    strategy: 'ssr',
    revalidate: 60,
  },

  cache: {
    routes: true,
    css: true,
  },

  debug: {
    overlay: true,
    traces: true,
  },

  middleware: {
    workers: 2,
  },
  image: {
    optimize: true,
    quality: 82,
    lossless: false,
    workers: 0,
  },

  plugins: [...demoPlugins, realtime()],
}

export default config(settings)
