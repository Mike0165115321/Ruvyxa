# @ruvyxa/testing

Small framework-shaped test doubles for Ruvyxa loaders, actions, and caches. They run in Node,
Vitest, or Jest without starting a Ruvyxa server.

```ts
import { mockAction, mockCache, mockLoader } from '@ruvyxa/testing'

const cache = mockCache({ 'posts:list': [] })
const loadPosts = mockLoader(async ({ params }) => ({ params }))
const savePost = mockAction(async ({ input, invalidate }) => {
  invalidate('posts')
  return input
})
```
