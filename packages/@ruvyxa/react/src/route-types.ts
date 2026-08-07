/**
 * Compile-time route checking for `<Link href>` and the imperative router.
 *
 * Nothing here exists at runtime. `ruvyxa dev`, `ruvyxa build`, and
 * `ruvyxa check` write `.ruvyxa/types/routes.d.ts`, which augments
 * {@link RuvyxaRouteRegistry} with one key per discovered page route. The types
 * below turn that registry into the set of URLs an application may navigate to.
 *
 * The registry is empty until that file is generated *and* included by the
 * project's `tsconfig.json`. In that state {@link RouteHref} collapses to
 * `string`, so a project that never opts in — and every project that predates
 * this feature — type-checks exactly as it did before.
 */

/**
 * Route patterns known to the type system, keyed by pattern (`/blog/[slug]`).
 *
 * Augmented by the generated `.ruvyxa/types/routes.d.ts`. Declaring keys by
 * hand also works, which is what a test fixture or a route served entirely by
 * middleware would do:
 *
 * ```ts
 * declare module '@ruvyxa/react/routes' {
 *   interface RuvyxaRouteRegistry {
 *     '/health': true
 *   }
 * }
 * ```
 *
 * The augmentation must target `@ruvyxa/react/routes`, the subpath that
 * resolves to *this* file. Augmenting `@ruvyxa/react` would declare a second,
 * unrelated interface in the barrel module: re-exports do not participate in
 * declaration merging, so the keys would never reach {@link RouteHref}.
 */
// The empty interface is the extension point: an augmentation merges into it.
// eslint-disable-next-line @typescript-eslint/no-empty-object-type
export interface RuvyxaRouteRegistry {}

/** Every generated route pattern, as a union of string literals. */
export type RoutePattern = Extract<keyof RuvyxaRouteRegistry, string>

/**
 * Drop one trailing slash, so an optional catch-all also matches its parent.
 *
 * `/docs/[[...path]]` serves `/docs` as well as `/docs/a/b`, and the head
 * captured before the segment is `/docs/`. The root is left alone: `/` is a
 * real path, not a separator with nothing in front of it.
 */
type WithoutTrailingSlash<S extends string> = S extends '/'
  ? '/'
  : S extends `${infer Head}/`
    ? Head
    : S

/**
 * Expand a route pattern into the URLs it serves.
 *
 * `[slug]`, `[...rest]`, and `[[...rest]]` each become `${string}`; an optional
 * catch-all additionally yields the pattern with the whole segment removed.
 * The branches are ordered longest-delimiter first, because `[[...` and `[...`
 * both also match the plainer `[` case.
 */
export type RouteFromPattern<P extends string> =
  P extends `${infer Head}[[...${string}]]${infer Tail}`
    ? | `${Head}${string}${RouteFromPattern<Tail>}`
      | RouteFromPattern<`${WithoutTrailingSlash<Head>}${Tail}`>
    : P extends `${infer Head}[...${string}]${infer Tail}`
      ? `${Head}${string}${RouteFromPattern<Tail>}`
      : P extends `${infer Head}[${string}]${infer Tail}`
        ? `${Head}${string}${RouteFromPattern<Tail>}`
        : P

/** Concrete URLs served by the application's own routes. */
export type KnownRoute = RouteFromPattern<RoutePattern>

/**
 * Destinations that are valid but are not application routes.
 *
 * `<Link>` renders a real anchor, so an absolute URL, a `mailto:`, and an
 * in-page anchor all remain legitimate `href` values once route checking is on.
 * Any scheme is accepted: the point of route checking is to catch a mistyped
 * *internal* path, and no route pattern contains a colon.
 */
export type ExternalHref = `${string}:${string}` | `#${string}` | `//${string}`

/**
 * A URL `<Link href>` and the router accept.
 *
 * Resolves to `string` while {@link RuvyxaRouteRegistry} is empty. Once routes
 * are generated it is the known routes, their query and hash variants, and
 * {@link ExternalHref}.
 */
export type RouteHref = [RoutePattern] extends [never]
  ? string
  : KnownRoute | `${KnownRoute}?${string}` | `${KnownRoute}#${string}` | ExternalHref

/**
 * Accept a URL that only exists at runtime.
 *
 * A path assembled from data is a `string`, and `string` is not assignable to a
 * union of literals. This is the escape hatch — it asserts rather than
 * validates, so prefer a template built from a literal pattern where possible.
 *
 * @example
 * ```tsx
 * <Link href={route(record.canonicalUrl)}>Open</Link>
 * ```
 */
export function route(href: string): RouteHref {
  return href as RouteHref
}
