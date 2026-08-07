#!/usr/bin/env node
import { STARTER_TEMPLATES, createRuvyxaApp, detectPackageManager } from '../dist/index.js'

const args = process.argv.slice(2)
if (args.includes('--help') || args.includes('-h')) {
  console.log(`Usage: create-ruvyxa [directory] [--template ${STARTER_TEMPLATES.join('|')}]`)
  process.exit(0)
}
const templateArg = args.find((arg) => arg.startsWith('--template='))
const templateIndex = args.findIndex((arg) => arg === '--template' || arg === '-t')
const templateValue = templateIndex >= 0 ? args[templateIndex + 1] : undefined
const template = templateArg?.slice('--template='.length) ?? templateValue
const missingTemplate =
  templateArg === '--template=' ||
  (templateIndex >= 0 && (!templateValue || templateValue.startsWith('-')))
const target =
  args.find(
    (arg, index) => !arg.startsWith('-') && index !== (templateIndex >= 0 ? templateIndex + 1 : -1),
  ) ?? 'my-ruvyxa-app'
const color = process.stdout.isTTY && !process.env.NO_COLOR

// A muted dark-editor palette, used throughout: true color where the terminal advertises
// it, otherwise the nearest xterm-256 slot so it still reads the same in a 256-color one.
const truecolor = /(^|[^a-z])(truecolor|24bit)([^a-z]|$)/i.test(process.env.COLORTERM ?? '')

function ink(hex, xterm256) {
  if (!truecolor) return `38;5;${xterm256}`
  const rgb = Number.parseInt(hex, 16)
  return `38;2;${(rgb >> 16) & 255};${(rgb >> 8) & 255};${rgb & 255}`
}

const CYAN = ink('56b6c2', 73)
const GREEN = ink('98c379', 114)
const PURPLE = ink('c678dd', 176)
const RED = ink('e06c75', 174)
const COMMENT = ink('5c6370', 240)
const cyan = (value) => format(value, CYAN)
const green = (value) => format(value, GREEN)
const magenta = (value) => format(value, PURPLE)
const gray = (value) => format(value, COMMENT)
const red = (value) => format(value, RED)
const bold = (value) => format(value, '1')
const dim = (value) => format(value, '2')

function format(value, code) {
  return color ? `\x1b[${code}m${value}\x1b[0m` : value
}

if (color) {
  process.on('exit', () => process.stdout.write('\x1b[?25h'))
}

// The Ruvyxa octopus, pixel-for-pixel from examples/demo/app/components/ruvyxa-runner.tsx:
// black eyes, a purple body, four tentacles that lift left/right as it "runs" — same 4 gait
// frames as RUNNER_FRAMES in that file (idle, step-left, idle, step-right).
const RUNNER_SPRITE = [
  '00111100',
  '01111110',
  '11K11K11',
  '01111110',
  '00111100',
  '11111111',
  '10100101',
  '01011010',
]
const RUNNER_SPRITE_STEP_LEFT = [
  '00111100',
  '01111110',
  '11K11K11',
  '01111110',
  '00111100',
  '11111111',
  '10100101',
  '10010101',
]
const RUNNER_SPRITE_STEP_RIGHT = [
  '00111100',
  '01111110',
  '11K11K11',
  '01111110',
  '00111100',
  '11111111',
  '10100101',
  '10101001',
]
const RUNNER_FRAMES = [
  RUNNER_SPRITE,
  RUNNER_SPRITE_STEP_LEFT,
  RUNNER_SPRITE,
  RUNNER_SPRITE_STEP_RIGHT,
]
const MASCOT_PURPLE = 141
const MASCOT_BLACK = 16

function renderMascot(sprite) {
  if (!color) return []
  return sprite.map((row) => {
    let line = ''
    for (const cell of row) {
      if (cell === '0') {
        line += '  '
      } else {
        const bg = cell === 'K' ? MASCOT_BLACK : MASCOT_PURPLE
        line += `\x1b[48;5;${bg}m  \x1b[0m`
      }
    }
    return line
  })
}

function bannerLines(sprite, status) {
  const mascot = renderMascot(sprite)
  const info = []
  info[3] = bold(magenta('RUVYXA'))
  info[4] = dim('create-ruvyxa')
  if (!mascot.length) {
    return [`  ${bold(magenta('RUVYXA'))} ${dim('create-ruvyxa')}`, '', `  ${status}`]
  }
  const lines = mascot.map((row, i) => `  ${row}  ${info[i] ?? ''}`)
  lines.push('', `  ${status}`)
  return lines
}

// The starters do not share one layout — `blog` has routes `minimal` lacks, `api-backend`
// has no page components at all — so the summary is rendered from the files that were
// actually written rather than from an assumed structure.
const TREE_MAX_ENTRIES = 24

// One hue per role, the way a syntax highlighter separates token kinds: red directories,
// blue markup, cyan modules, purple styles, amber config, green assets, foreground-gray
// docs, muted-gray dotfiles and branches. The project root is a directory too, so it
// reads in the same red as the rest of them.
const TREE_DIR = `1;${RED}`
const TREE_MARKUP = ink('61afef', 75)
const TREE_MODULE = CYAN
const TREE_STYLE = PURPLE
const TREE_CONFIG = ink('e5c07b', 180)
const TREE_ASSET = GREEN
const TREE_DOC = ink('abb2bf', 145)
const TREE_DOTFILE = ink('7f848e', 244)
const TREE_OTHER = ink('abb2bf', 145)
const TREE_BRANCH = COMMENT

function colorizeEntry(name, isDirectory) {
  if (isDirectory) return format(`${name}/`, TREE_DIR)
  if (/^ruvyxa\.config\.[cm]?[jt]s$/.test(name)) return bold(format(name, TREE_CONFIG))
  if (/^(package(-lock)?\.json|tsconfig(\..+)?\.json|.*\.config\.[cm]?[jt]s)$/.test(name)) {
    return format(name, TREE_CONFIG)
  }
  if (/\.[cm]?[jt]sx$/.test(name)) return format(name, TREE_MARKUP)
  if (/\.[cm]?[jt]s$/.test(name)) return format(name, TREE_MODULE)
  if (/\.(css|scss|sass|less)$/.test(name)) return format(name, TREE_STYLE)
  if (/\.(md|mdx|txt)$/.test(name)) return format(name, TREE_DOC)
  if (/\.(png|jpe?g|gif|svg|webp|avif|ico|woff2?|ttf|otf|mp4|webm)$/.test(name)) {
    return format(name, TREE_ASSET)
  }
  if (/\.(json|jsonc|ya?ml|toml)$/.test(name)) return format(name, TREE_CONFIG)
  if (name.startsWith('.')) return format(name, TREE_DOTFILE)
  return format(name, TREE_OTHER)
}

/** Build a nested tree from project-relative POSIX file paths. */
function buildTree(files) {
  const root = new Map()
  for (const file of files) {
    let node = root
    const segments = file.split('/')
    segments.forEach((segment, index) => {
      const isFile = index === segments.length - 1
      if (!node.has(segment)) node.set(segment, isFile ? null : new Map())
      if (!isFile) node = node.get(segment)
    })
  }
  return root
}

/** Directories first, then files, each alphabetically — stable across platforms. */
function sortedEntries(node) {
  return [...node.entries()].sort(([leftName, left], [rightName, right]) => {
    const leftIsDir = left !== null
    const rightIsDir = right !== null
    if (leftIsDir !== rightIsDir) return leftIsDir ? -1 : 1
    return leftName.localeCompare(rightName)
  })
}

function treeLines(node, prefix = '', budget = { remaining: TREE_MAX_ENTRIES, hidden: 0 }) {
  const entries = sortedEntries(node)
  const lines = []
  for (const [index, [name, child]] of entries.entries()) {
    if (budget.remaining <= 0) {
      budget.hidden += countEntries(node, index)
      break
    }
    budget.remaining -= 1
    const isLast = index === entries.length - 1
    const connector = isLast ? '└─ ' : '├─ '
    lines.push(`${prefix}${format(connector, TREE_BRANCH)}${colorizeEntry(name, child !== null)}`)
    if (child !== null) {
      lines.push(
        ...treeLines(child, `${prefix}${format(isLast ? '   ' : '│  ', TREE_BRANCH)}`, budget),
      )
    }
  }
  return lines
}

/** Count the entries from `startIndex` onward, including everything nested below them. */
function countEntries(node, startIndex) {
  return sortedEntries(node)
    .slice(startIndex)
    .reduce((total, [, child]) => total + 1 + (child === null ? 0 : countEntries(child, 0)), 0)
}

const MASCOT_FRAME_MS = 160
const MASCOT_MIN_LOOPS = 1

function startMascotSpinner(label) {
  const idleFrame = bannerLines(RUNNER_FRAMES[0], label).join('\n')
  if (!color) {
    console.log(idleFrame)
    return async () => {}
  }
  const spinner = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
  let gait = 0
  let spin = 0
  const startedAt = Date.now()
  process.stdout.write('\x1b[?25l')
  process.stdout.write('\x1b[s')
  const redraw = (status) => {
    process.stdout.write('\x1b[u\x1b[J')
    process.stdout.write(bannerLines(RUNNER_FRAMES[gait], status).join('\n'))
  }
  redraw(`${cyan(spinner[0])} ${label}`)
  const timer = setInterval(() => {
    gait = (gait + 1) % RUNNER_FRAMES.length
    spin = (spin + 1) % spinner.length
    redraw(`${cyan(spinner[spin])} ${label}`)
  }, MASCOT_FRAME_MS)
  return async (finalLabel) => {
    const minDuration = RUNNER_FRAMES.length * MASCOT_FRAME_MS * MASCOT_MIN_LOOPS
    const elapsed = Date.now() - startedAt
    if (elapsed < minDuration) {
      await new Promise((resolve) => setTimeout(resolve, minDuration - elapsed))
    }
    clearInterval(timer)
    redraw(`${green('✓')} ${finalLabel ?? label}`)
    process.stdout.write('\n')
    process.stdout.write('\x1b[?25h')
  }
}

try {
  if (missingTemplate) {
    throw new Error(
      'Starter template name is required.\n' + `  Choose one of: ${STARTER_TEMPLATES.join(', ')}`,
    )
  }

  console.log('')
  const stopSpinner = startMascotSpinner(`Scaffolding ${bold(target)}...`)
  const result = await createRuvyxaApp(target, template ? { template } : undefined)
  await stopSpinner(`Created ${bold(cyan(target))}`)

  const pm = detectPackageManager()

  console.log('')
  console.log(`  ${gray('starter:')} ${result.template}`)
  console.log('')
  console.log(`  ${bold('Project')} ${dim(`(${result.files.length} files)`)}`)
  console.log('')
  console.log(`  ${colorizeEntry(target, true)}`)
  const budget = { remaining: TREE_MAX_ENTRIES, hidden: 0 }
  for (const line of treeLines(buildTree(result.files), '  ', budget)) {
    console.log(line)
  }
  if (budget.hidden > 0) {
    console.log(`  ${dim(`… and ${budget.hidden} more`)}`)
  }
  console.log('')
  console.log(`  ${bold('Next steps')} ${dim(`(detected: ${pm.name})`)}`)
  console.log('')
  console.log(`    ${cyan('cd')} ${target}`)
  console.log(`    ${cyan(pm.install)}`)
  console.log(`    ${cyan(pm.dev)}`)
  console.log('')
  console.log(
    `  ${format('Clarity over cleverness. Speed by default. Control that stays yours.', `1;${PURPLE}`)}`,
  )
  console.log('')
} catch (err) {
  const message = err instanceof Error ? err.message : String(err)
  console.error('')
  console.error(`  ${red('[error]')} ${message}`)
  console.error('')
  process.exit(1)
}
