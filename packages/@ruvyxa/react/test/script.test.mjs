/**
 * `<Script>`: what reaches the server HTML, and what the client injects.
 *
 * The strategy split is the whole point of the component, so both halves are
 * covered — `renderToStaticMarkup` for the server output, and `injectScript`
 * against a stub document for the client. A stub is enough because everything
 * worth asserting is a DOM call the component makes or declines to make.
 */

import assert from 'node:assert/strict'
import { describe, it, beforeEach } from 'node:test'

import { createElement } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'

import { Script, injectScript, resetInjectedScripts } from '../dist/script.js'

function stubDocument() {
  const appended = []
  return {
    appended,
    createElement() {
      const listeners = {}
      return {
        attributes: {},
        listeners,
        id: '',
        src: '',
        textContent: '',
        setAttribute(name, value) {
          this.attributes[name] = value
        },
        addEventListener(type, handler) {
          listeners[type] = handler
        },
      }
    },
    body: {
      append(node) {
        appended.push(node)
      },
    },
  }
}

function inject(doc, options) {
  injectScript(doc, {
    inlineCode: null,
    attributes: {},
    onLoad: () => {},
    onError: () => {},
    ...options,
  })
}

beforeEach(() => {
  resetInjectedScripts()
})

describe('server output', () => {
  it('renders beforeInteractive scripts into the HTML', () => {
    const html = renderToStaticMarkup(
      createElement(Script, { src: 'https://cdn.example.com/a.js', strategy: 'beforeInteractive' }),
    )
    assert.match(html, /<script src="https:\/\/cdn\.example\.com\/a\.js"/)
  })

  it('renders inline beforeInteractive code', () => {
    const html = renderToStaticMarkup(
      createElement(
        Script,
        { id: 'consent', strategy: 'beforeInteractive' },
        'window.__consent = true',
      ),
    )
    assert.match(html, /window\.__consent = true/)
  })

  it('emits nothing for the deferred strategies', () => {
    // They are appended by an effect after hydration; putting them in the
    // server HTML too would load each script twice.
    for (const strategy of ['afterInteractive', 'lazyOnload']) {
      const html = renderToStaticMarkup(
        createElement(Script, { src: 'https://cdn.example.com/a.js', strategy }),
      )
      assert.equal(html, '', `${strategy} must not reach the server HTML`)
    }
  })

  it('defaults to afterInteractive', () => {
    const html = renderToStaticMarkup(
      createElement(Script, { src: 'https://cdn.example.com/a.js' }),
    )
    assert.equal(html, '')
  })
})

describe('client injection', () => {
  it('appends one script with its attributes', () => {
    const doc = stubDocument()
    inject(doc, {
      key: 'a',
      src: 'https://cdn.example.com/a.js',
      id: 'a',
      attributes: { async: true, crossOrigin: 'anonymous', defer: false },
    })

    assert.equal(doc.appended.length, 1)
    const [element] = doc.appended
    assert.equal(element.src, 'https://cdn.example.com/a.js')
    assert.equal(element.id, 'a')
    assert.equal(element.attributes.async, '', 'a boolean attribute is present, not "true"')
    assert.equal(element.attributes['cross-origin'], 'anonymous')
    assert.ok(!('defer' in element.attributes), 'a false boolean attribute is omitted entirely')
  })

  it('injects a given key only once', () => {
    // Two routes rendering the same analytics tag must not load it twice, and a
    // navigation back to a route must not re-run its script.
    const doc = stubDocument()
    inject(doc, { key: 'analytics', src: 'https://cdn.example.com/a.js' })
    inject(doc, { key: 'analytics', src: 'https://cdn.example.com/a.js' })
    assert.equal(doc.appended.length, 1)
  })

  it('releases the key when the script fails, so a later render can retry', () => {
    const doc = stubDocument()
    const errors = []
    injectScript(doc, {
      key: 'flaky',
      src: 'https://cdn.example.com/flaky.js',
      inlineCode: null,
      attributes: {},
      onLoad: () => {},
      onError: (error) => errors.push(error),
    })
    doc.appended[0].listeners.error('boom')

    assert.deepEqual(errors, ['boom'])
    inject(doc, { key: 'flaky', src: 'https://cdn.example.com/flaky.js' })
    assert.equal(doc.appended.length, 2, 'a failed script must not hold its key forever')
  })

  it('reports load synchronously for inline code', () => {
    // There is no load event on an inline script; a caller waiting on `onLoad`
    // would otherwise wait forever.
    const doc = stubDocument()
    let loaded = false
    injectScript(doc, {
      key: 'inline',
      inlineCode: 'window.x = 1',
      attributes: {},
      onLoad: () => {
        loaded = true
      },
      onError: () => {},
    })
    assert.equal(doc.appended[0].textContent, 'window.x = 1')
    assert.ok(loaded)
  })
})
