import { definePlugin } from 'ruvyxa/plugin'

export default definePlugin({
  name: '__PLUGIN_NAME__',
  register({ http }) {
    http.onResponse({
      match: ['/*'],
      handler({ response }) {
        const headers = new Headers(response.headers)
        headers.set('x-__PLUGIN_NAME__', 'active')
        return new Response(response.body, {
          status: response.status,
          statusText: response.statusText,
          headers,
        })
      },
    })
  },
})
