/**
 * What `RouteHref` accepts and rejects once routes are generated.
 *
 * Every rejection is asserted with `@ts-expect-error`, so a single
 * zero-diagnostic `tsc` run proves both directions: a line that was supposed to
 * fail but compiles turns the directive itself into an error.
 */

import type { RouteHref } from '@ruvyxa/react/routes'
import { route } from '@ruvyxa/react/routes'

// Static routes.
export const root: RouteHref = '/'
export const about: RouteHref = '/about'

// A required dynamic segment accepts any value in that position.
export const post: RouteHref = '/blog/hello-world'
// ...including one built from data at compile time.
declare const slug: string
export const interpolated: RouteHref = `/blog/${slug}`
// ...and keeps the suffix that follows it significant.
export const edit: RouteHref = '/blog/hello-world/edit'

// Catch-all and optional catch-all.
export const files: RouteHref = '/files/a/b/c'
export const docsChild: RouteHref = '/docs/getting-started'
// The optional catch-all also serves its own parent.
export const docsRoot: RouteHref = '/docs'

// Two segments in one pattern.
export const item: RouteHref = '/shop/shoes/oxford'

// Query strings and hashes.
export const query: RouteHref = '/about?ref=nav'
export const hash: RouteHref = '/about#team'
export const both: RouteHref = '/blog/hello?draft=1'

// External destinations stay legal: `<Link>` renders a real anchor.
export const external: RouteHref = 'https://example.com/docs'
export const mail: RouteHref = 'mailto:hi@example.com'
export const anchor: RouteHref = '#top'
export const protocolRelative: RouteHref = '//cdn.example.com/x'

// A route that does not exist.
// @ts-expect-error unknown route
export const typo: RouteHref = '/abuot'

// A near-miss on a dynamic route's static prefix.
// @ts-expect-error `/blogs` is not a route; `/blog/[slug]` is
export const wrongPrefix: RouteHref = '/blogs/hello'

// Two documented limitations, recorded here rather than left to be discovered.
//
// A dynamic segment expands to `${string}`, and a template literal type cannot
// say "any string except one containing a slash". So a single `[slug]` also
// matches a multi-segment value, and the pattern's own literal text matches
// itself. Both are accepted, and both are the same trade-off Next.js makes.
// What the check does catch is the far more common mistake: a path whose
// *static* parts are wrong.
export const extraSegment: RouteHref = '/blog/hello/publish'
export const literalPattern: RouteHref = '/blog/[slug]'

// A bare `string` is not assignable, which is what makes the check worth
// having — and why `route()` exists.
declare const fromData: string
// @ts-expect-error a runtime string must go through `route()`
export const unchecked: RouteHref = fromData
export const checked: RouteHref = route(fromData)
