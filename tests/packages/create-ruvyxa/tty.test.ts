/**
 * In-place redrawing for the scaffolder's banner and template picker.
 *
 * The regression these guard against is visible rather than thrown: the banner
 * animated into a vertical stack of mascots, one per frame, instead of
 * replacing itself. The cause was `\x1b[s`/`\x1b[u` (save/restore cursor),
 * which store an absolute screen position that stops being correct the moment
 * the terminal scrolls — which drawing a ten-line banner near the bottom of a
 * viewport always does.
 *
 * A fake stream is enough to catch it: the bug is entirely in which escape
 * sequences are written, and every assertion below is about that.
 */

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'

import {
  createFrame,
  physicalRows,
  rewindSequence,
  stripAnsi,
  visibleWidth,
  type FrameStream,
} from '../../../packages/create-ruvyxa/dist/tty.js'

/** A stream that records what was written and reports a fixed size. */
function fakeStream(columns = 80, rows = 24) {
  const chunks: string[] = []
  const stream: FrameStream & { chunks: string[]; output(): string } = {
    chunks,
    columns,
    rows,
    write(chunk: string) {
      chunks.push(chunk)
      return true
    },
    output: () => chunks.join(''),
  }
  return stream
}

/**
 * Replay written output against a fake screen, so assertions can be made about
 * what the user ends up seeing rather than about the escape codes alone.
 *
 * Supports exactly the sequences this module emits: cursor-up, carriage
 * return, erase-to-end-of-screen, and cursor show/hide.
 */
function renderScreen(output: string): string[] {
  const screen: string[] = ['']
  let row = 0
  let column = 0

  const put = (text: string) => {
    while (screen.length <= row) screen.push('')
    const line = screen[row].padEnd(column, ' ')
    screen[row] = line.slice(0, column) + text + line.slice(column + text.length)
    column += text.length
  }

  let index = 0
  while (index < output.length) {
    const character = output[index]
    if (character === '\n') {
      row += 1
      column = 0
      while (screen.length <= row) screen.push('')
      index += 1
      continue
    }
    if (character === '\r') {
      column = 0
      index += 1
      continue
    }
    if (character !== '\x1b') {
      put(character)
      index += 1
      continue
    }

    const match = /^\x1b\[([0-9;?]*)([A-Za-z])/.exec(output.slice(index))
    assert.ok(match, `unrecognised escape sequence at ${index}`)
    const [sequence, parameters, final] = match
    index += sequence.length
    if (final === 'A') {
      row = Math.max(0, row - (Number(parameters) || 1))
    } else if (final === 'J' && (parameters === '' || parameters === '0')) {
      screen[row] = screen[row].slice(0, column)
      screen.length = row + 1
    } else if (final === 'l' || final === 'h' || final === 'm') {
      // Cursor visibility and colour occupy no cells.
    } else {
      assert.fail(`unexpected sequence ${JSON.stringify(sequence)}`)
    }
  }
  return screen.map((line) => line.trimEnd())
}

describe('visible width', () => {
  it('ignores styling, which occupies no columns', () => {
    assert.equal(visibleWidth('\x1b[36mvalue\x1b[0m'), 5)
    assert.equal(stripAnsi('\x1b[1;35mRUVYXA\x1b[0m'), 'RUVYXA')
  })

  it('counts a background-coloured block as the cells it paints', () => {
    // How the mascot is drawn: two spaces with a background colour per pixel.
    assert.equal(visibleWidth('\x1b[48;5;141m  \x1b[0m'.repeat(8)), 16)
  })
})

describe('physical rows', () => {
  it('counts an empty line as one row', () => {
    assert.equal(physicalRows(['a', '', 'b'], 80), 3)
  })

  it('counts a wrapped line as the rows it really occupies', () => {
    // Walking back by the logical count would leave the overflow on screen.
    assert.equal(physicalRows(['x'.repeat(200)], 80), 3)
    assert.equal(physicalRows(['x'.repeat(80)], 80), 1)
    assert.equal(physicalRows(['x'.repeat(81)], 80), 2)
  })

  it('measures wrapping on visible width, not on styled length', () => {
    // A short but heavily styled line is one row; counting escape bytes would
    // make it look like several and rewind too far, eating output above.
    const styled = '\x1b[48;5;141m  \x1b[0m'.repeat(8)
    assert.equal(physicalRows([styled], 80), 1)
  })
})

describe('rewind sequence', () => {
  it('walks up one row fewer than it wrote, because it never wrote a newline', () => {
    assert.equal(rewindSequence(10), '\x1b[9A\r\x1b[0J')
  })

  it('only returns to column zero for a single row', () => {
    assert.equal(rewindSequence(1), '\r\x1b[0J')
  })

  it('does nothing for an empty frame', () => {
    assert.equal(rewindSequence(0), '')
  })

  it('never uses absolute save/restore', () => {
    // The whole bug: `\x1b[u` restores a position that scrolling invalidated.
    for (const rows of [1, 2, 10, 100]) {
      assert.doesNotMatch(rewindSequence(rows), /\x1b\[[su]/)
    }
  })
})

describe('a redrawn frame', () => {
  // No trailing spaces: `renderScreen` trims them, because a terminal cannot
  // distinguish a blank cell that was written from one that never was.
  const banner = (status: string) => ['  ####', ' ######', '  ####', '', `  ${status}`]

  it('replaces the previous frame instead of stacking copies', () => {
    // The reported bug, reproduced end to end: four animation frames must leave
    // one banner on screen, not four.
    const stream = fakeStream()
    const frame = createFrame(stream, true)
    for (const status of ['one', 'two', 'three', 'four']) frame.render(banner(status))

    const screen = renderScreen(stream.output())
    assert.deepEqual(screen, banner('four'))
    assert.equal(
      screen.filter((line) => line.includes('######')).length,
      1,
      'exactly one mascot should remain on screen',
    )
  })

  it('leaves the last frame in place when it finishes', () => {
    const stream = fakeStream()
    const frame = createFrame(stream, true)
    frame.render(banner('working'))
    frame.finish(banner('done'))

    const screen = renderScreen(stream.output())
    assert.deepEqual(screen.slice(0, banner('done').length), banner('done'))
    assert.equal(screen.at(-1), '', 'output continues on a fresh line below')
  })

  it('erases everything when it is cleared', () => {
    // The template picker: its answer is echoed elsewhere, so no trace remains.
    const stream = fakeStream()
    const frame = createFrame(stream, true)
    frame.render(['  ? Select a starter template', '', '  ❯ minimal', '    blog'])
    frame.clear()

    assert.deepEqual(renderScreen(stream.output()), [''])
  })

  it('restores the cursor it hid, on both exits', () => {
    for (const release of ['finish', 'clear'] as const) {
      const stream = fakeStream()
      const frame = createFrame(stream, true)
      frame.render(['x'])
      frame[release]()
      assert.match(stream.output(), /\x1b\[\?25l/, `${release} should have hidden the cursor`)
      assert.match(stream.output(), /\x1b\[\?25h/, `${release} must show the cursor again`)
    }
  })

  it('rewinds by the rows a wrapped frame really occupied', () => {
    // A narrow terminal wraps the status line. Rewinding by the logical line
    // count would leave the wrapped remainder behind on every frame.
    const stream = fakeStream(20)
    const frame = createFrame(stream, true)
    const long = ['  ####  ', `  ${'status '.repeat(6)}`]
    frame.render(long)
    frame.render(long)

    const screen = renderScreen(stream.output())
    assert.equal(
      screen.filter((line) => line.includes('####')).length,
      1,
      'the wrapped frame must still replace itself exactly once',
    )
  })
})

describe('a frame taller than the viewport', () => {
  it('reports that it cannot be redrawn', () => {
    // Its top row scrolls out of reach, so there is nothing to rewind to. The
    // caller draws once instead of animating into the stack this bug produced.
    const stream = fakeStream(80, 10)
    const frame = createFrame(stream, true)
    const tall = Array.from({ length: 12 }, (_, row) => `row ${row}`)

    assert.equal(frame.canRedraw(tall), false)
    assert.equal(frame.canRedraw(['one', 'two']), true)
  })

  it('accounts for wrapping when deciding', () => {
    const stream = fakeStream(20, 10)
    const frame = createFrame(stream, true)
    // Five logical lines, but each wraps to three rows: fifteen in total.
    assert.equal(frame.canRedraw(Array.from({ length: 5 }, () => 'x'.repeat(50))), false)
  })
})
