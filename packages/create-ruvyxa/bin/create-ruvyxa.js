#!/usr/bin/env node
import { createRuvyxaApp, detectPackageManager } from '../dist/index.js'

const args = process.argv.slice(2)
if (args.includes('--help') || args.includes('-h')) {
  console.log('Usage: create-ruvyxa [directory] [--template minimal|blog|crud|api-backend]')
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
const cyan = (value) => format(value, '36')
const green = (value) => format(value, '32')
const yellow = (value) => format(value, '33')
const magenta = (value) => format(value, '35')
const blue = (value) => format(value, '34')
const gray = (value) => format(value, '90')
const red = (value) => format(value, '31')
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

function startMascotSpinner(label) {
  const idleFrame = bannerLines(RUNNER_FRAMES[0], label).join('\n')
  if (!color) {
    console.log(idleFrame)
    return () => {}
  }
  const spinner = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
  let gait = 0
  let spin = 0
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
  }, 160)
  return (finalLabel) => {
    clearInterval(timer)
    redraw(`${green('✓')} ${finalLabel ?? label}`)
    process.stdout.write('\n')
    process.stdout.write('\x1b[?25h')
  }
}

try {
  if (missingTemplate) {
    throw new Error(
      'Starter template name is required.\n' + '  Choose one of: minimal, blog, crud, api-backend',
    )
  }

  console.log('')
  const stopSpinner = startMascotSpinner(`Scaffolding ${bold(target)}...`)
  await createRuvyxaApp(target, template ? { template } : undefined)
  stopSpinner(`Created ${bold(cyan(target))}`)

  const pm = detectPackageManager()

  console.log('')
  console.log(`  ${gray('starter:')} ${template ?? 'minimal'}`)
  console.log('')
  console.log(`  ${bold('Project')}`)
  console.log(`    ${gray('app/')}${blue('page.tsx')}`)
  console.log(`    ${gray('app/')}${blue('layout.tsx')}`)
  console.log(`    ${gray('app/')}${magenta('globals.css')}`)
  console.log(`    ${yellow('ruvyxa.config.ts')}`)
  console.log(`    ${gray('AGENTS.md')}`)
  console.log(`    ${gray('CLAUDE.md')}`)
  console.log('')
  console.log(`  ${bold('Next steps')} ${dim(`(detected: ${pm.name})`)}`)
  console.log(`    ${cyan('cd')} ${target}`)
  console.log(`    ${cyan(pm.install)}`)
  console.log(`    ${cyan(pm.dev)}`)
  console.log('')
  console.log(`  ${dim('Happy shipping! 🚀')}`)
  console.log('')
} catch (err) {
  const message = err instanceof Error ? err.message : String(err)
  console.error('')
  console.error(`  ${red('[error]')} ${message}`)
  console.error('')
  process.exit(1)
}
