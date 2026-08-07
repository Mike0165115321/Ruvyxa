import { revalidatePath } from 'ruvyxa/server'

/**
 * A CMS webhook, reduced to its essentials.
 *
 * `revalidatePath` reaches the server with this response, so a client that
 * navigates after a 200 cannot arrive before the cached document is gone.
 */
export async function POST({ request }: { request: Request }) {
  const { path } = (await request.json()) as { path?: unknown }
  if (typeof path !== 'string' || !path.startsWith('/')) {
    return Response.json({ error: 'path must be an absolute URL path' }, { status: 400 })
  }

  revalidatePath(path)
  return Response.json({ revalidated: path })
}
