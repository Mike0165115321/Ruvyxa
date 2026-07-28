import { bundleBudget } from 'ruvyxa/plugins'

import buildPipeline from './build-pipeline'
import pageObservability from './page-observability'
import renderModeBadges from './render-mode-badges'

export const demoPlugins = [
  pageObservability,
  renderModeBadges,
  buildPipeline,
  bundleBudget({ maxChunkKb: 1024, maxTotalKb: 4096 }),
]
