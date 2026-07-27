import { definePlugin } from 'ruvyxa/plugin'

export default definePlugin({
  name: '__PLUGIN_NAME__',
  headers: { 'x-__PLUGIN_NAME__': 'active' },
})
