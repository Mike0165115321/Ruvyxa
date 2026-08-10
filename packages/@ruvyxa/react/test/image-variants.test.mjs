import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import { Image } from '../dist/image.js'

// Render the component to a plain element tree. The compiled JSX targets the
// automatic runtime, so calling the function directly returns the <img>
// descriptor without a DOM.
function renderImage(props) {
  const element = Image(props)
  return element.props
}

describe('Image responsive srcset', () => {
  it('uses one static WebP even when sizes is present', () => {
    const { srcSet } = renderImage({
      src: '/hero.jpg',
      alt: '',
      width: 1000,
      height: 500,
      sizes: '100vw',
    })

    assert.equal(srcSet, undefined)
  })

  it('emits no auto srcset without a sizes hint', () => {
    const { srcSet } = renderImage({ src: '/hero.jpg', alt: '', width: 1000, height: 500 })
    assert.equal(srcSet, undefined)
  })

  it('leaves an explicit srcSet under author control (rewriting local URLs)', () => {
    const { srcSet } = renderImage({
      src: '/hero.jpg',
      alt: '',
      width: 1000,
      height: 500,
      sizes: '100vw',
      srcSet: '/a.png 1x, /b.png 2x',
    })
    assert.equal(srcSet, '/a.webp 1x, /b.webp 2x')
  })

  it('does not fabricate static variants for a remote or already-optimized source', () => {
    assert.equal(
      renderImage({
        src: 'https://cdn.example/x.jpg',
        alt: '',
        width: 1000,
        height: 500,
        sizes: '100vw',
      }).srcSet,
      undefined,
    )
    assert.equal(
      renderImage({ src: '/logo.svg', alt: '', width: 1000, height: 500, sizes: '100vw' }).srcSet,
      undefined,
    )
  })

  it('skips auto srcset when a loader or unoptimized owns the URL', () => {
    assert.equal(
      renderImage({
        src: '/hero.jpg',
        alt: '',
        width: 1000,
        height: 500,
        sizes: '100vw',
        unoptimized: true,
      }).srcSet,
      undefined,
    )
  })

  it('builds same-origin on-demand URLs without accepting remote sources', () => {
    const dynamic = renderImage({
      src: '/uploads/avatar.png?v=2',
      alt: 'Avatar',
      width: 828,
      height: 828,
      sizes: '100vw',
      quality: 75,
      dynamic: true,
    })
    assert.equal(dynamic.src, '/__ruvyxa/image?src=%2Fuploads%2Favatar.png&w=828&q=75')
    assert.match(dynamic.srcSet, /w=640&q=75 640w/)
    assert.equal(
      renderImage({
        src: 'https://cdn.example/avatar.png',
        alt: '',
        width: 828,
        height: 828,
        dynamic: true,
      }).src,
      'https://cdn.example/avatar.png',
    )
    const snapped = renderImage({
      src: '/uploads/hero.jpg',
      alt: 'Hero',
      width: 1600,
      height: 900,
      dynamic: true,
    })
    assert.match(snapped.src, /w=1920/)
  })
})
