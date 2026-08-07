/**
 * Behaviour with no generated route file: `RouteHref` must collapse to
 * `string`.
 *
 * This is the compatibility guarantee. Every project that predates typed
 * routes, and every project that never opts in, compiles through the same
 * `<Link href>` and `useRouter().push` signatures — so adding the types must
 * not narrow anything until a registry augmentation exists.
 */

import type { RouteHref, RoutePattern } from '@ruvyxa/react/routes'

declare const anything: string
export const arbitrary: RouteHref = anything
export const nonsense: RouteHref = '/not-a-real-route-anywhere'

// And the registry really is empty, rather than accidentally populated by some
// other declaration in the program.
export const noPatterns: [RoutePattern] extends [never] ? true : false = true
