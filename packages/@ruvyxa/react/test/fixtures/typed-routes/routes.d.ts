// Hand-written stand-in for a generated `.ruvyxa/types/routes.d.ts`.
//
// Kept byte-comparable with what `route_types_source` in
// `crates/ruvyxa_cli/src/route_types.rs` emits: if the Rust generator changes
// shape, the assertions in `route-types.test.mjs` catch the drift.

import type {} from '@ruvyxa/react/routes'

declare module '@ruvyxa/react/routes' {
  interface RuvyxaRouteRegistry {
    '/': true
    '/about': true
    '/blog/[slug]': true
    '/blog/[slug]/edit': true
    '/docs/[[...path]]': true
    '/files/[...path]': true
    '/shop/[category]/[item]': true
  }
}
