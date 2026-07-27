import { definePlugin } from 'ruvyxa/plugin'

/** Adds request/response metadata to the plugin showcase page. */
export default definePlugin({
  name: 'demo-page-observability',
  register({ http }) {
    http.onRequest({
      match: ['/plugin-lab'],
      handler({ request }) {
        const headers = new Headers(request.headers)
        headers.set('x-demo-plugin-request', 'active')
        return new Request(request, { headers })
      },
    })
    http.onResponse({
      match: ['/plugin-lab'],
      handler({ request, response }) {
        const headers = new Headers(response.headers)
        headers.set('x-demo-plugin-response', 'active')
        headers.set('x-demo-plugin-route', new URL(request.url).pathname)
        return new Response(response.body, {
          status: response.status,
          statusText: response.statusText,
          headers,
        })
      },
    })
  },
})
