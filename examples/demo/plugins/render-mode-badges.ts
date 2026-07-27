import { definePlugin } from 'ruvyxa/plugin'

const renderModes: Record<string, string> = {
  '/static-page': 'static',
  '/ssg-blog': 'ssg',
  '/isr-page': 'isr',
  '/csr-page': 'csr',
  '/ppr-page': 'ppr',
}

/** Labels different rendering strategies through the HTTP response socket. */
export default definePlugin({
  name: 'demo-render-mode-badges',
  register({ http }) {
    http.onResponse({
      match: ['/static-page', '/ssg-blog*', '/isr-page', '/csr-page', '/ppr-page'],
      handler({ request, response }) {
        const pathname = new URL(request.url).pathname
        const mode = Object.entries(renderModes).find(([prefix]) =>
          pathname.startsWith(prefix),
        )?.[1]
        if (!mode) return response

        const headers = new Headers(response.headers)
        headers.set('x-demo-render-mode', mode)
        return new Response(response.body, {
          status: response.status,
          statusText: response.statusText,
          headers,
        })
      },
    })
  },
})
