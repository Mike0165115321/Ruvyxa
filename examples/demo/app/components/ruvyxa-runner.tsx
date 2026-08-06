'use client'

import { useEffect, useRef } from 'react'

const WIDTH = 760
const HEIGHT = 240
const GROUND_Y = 196
const PIXEL = 3
const GRAVITY = 0.55
const JUMP_VELOCITY = -10.5
const MAX_AMMO = 3
const AMMO_REGEN = 70

const INK = '#171717'
const ACCENT = '#7c3aed'
const SPRITE_COLOR = '#8b5cf6'
const MUTED = '#a3a3a3'
const FAINT = '#e5e5e5'

// 8x8 Ruvyxa octopus runner: black eyes, a uniform purple body, and four swaying tentacles.
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

// Lift one inner tentacle while the opposite side stays planted.
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

// Crouched octopus pose — same eyes, body color, and four tentacles in a shorter hitbox.
const RUNNER_DUCK = ['0011111000', '011K11K110', '1111111110', '0111111100', '0101010100']

// Obstacles are scenery hazards, not characters, so they stay on a single static frame.
// Ground bugs — five silhouettes so a run never looks like the same three shapes repeating.
const BUG_SPRITES = [
  ['011010', '111111', '011110', '111111', '010010', '101101'],
  ['0110110', '1111111', '0111110', '1111111', '0100010', '1011101'],
  ['01101100', '11111110', '01111100', '11111110', '01000100', '10111011'],
  ['1010101', '0111110', '1111111', '0111110', '1010101'],
  ['00111100', '01111110', '11111111', '01111110', '10100101'],
]

// Flying errors — winged, forces a duck.
const ERROR_SPRITES = [
  ['10011001', '11011011', '01111110', '11111111', '01A11A10', '00100100'],
  ['01000010', '11011011', '01111110', '11A11A11', '01111110', '00100100'],
  ['10000001', '11100111', '01111110', '11A11A11', '01111110', '01011010'],
]

// Tall malware blocks — forces a jump.
const MALWARE_SPRITES = [
  ['011110', '111111', '1A11A1', '111111', '010010', '111111', '101101', '010010'],
  ['011110', '111111', '1A11A1', '111111', '111111', '101101', '111111', '010010'],
  [
    '0111110',
    '1111111',
    '11A1A11',
    '1111111',
    '0111110',
    '1111111',
    '1011101',
    '0111110',
    '0100010',
  ],
]

// Bosses. Every boss runs a four-frame loop; the frames are listed in playback order.

// Hooded hacker: hands work the keyboard while the visor flickers.
const HACKER_FRAMES = [
  [
    '0011111100',
    '0111111110',
    '1110000111',
    '110A00A011',
    '1110000111',
    '0111111110',
    '0011111100',
    '0111111110',
    '0110000110',
    '0100000010',
  ],
  [
    '0011111100',
    '0111111110',
    '1110000111',
    '110A00A011',
    '1110000111',
    '0111111110',
    '0011111100',
    '0111111110',
    '0110000110',
    '0010000100',
  ],
  [
    '0011111100',
    '0111111110',
    '1110000111',
    '1100000011',
    '111A00A111',
    '0111111110',
    '0011111100',
    '0111111110',
    '0110000110',
    '0100000010',
  ],
  [
    '0011111100',
    '0111111110',
    '1110000111',
    '110A00A011',
    '1110000111',
    '0111111110',
    '0011111100',
    '0111111110',
    '0110000110',
    '0011000110',
  ],
]

// Human error: a figure throwing its arms up mid-mistake.
const HUMAN_ERROR_FRAMES = [
  [
    '0001111000',
    '0011AA1100',
    '0011AA1100',
    '0001111000',
    '1001111001',
    '1111111111',
    '0011111100',
    '0001111000',
    '0011001100',
    '0110000110',
  ],
  [
    '1000000001',
    '1001111001',
    '0011AA1100',
    '0011AA1100',
    '0001111000',
    '0111111110',
    '0011111100',
    '0001111000',
    '0011001100',
    '0110000110',
  ],
  [
    '0001111000',
    '0011AA1100',
    '0011AA1100',
    '0001111000',
    '0111111110',
    '1111111111',
    '0011111100',
    '0001111000',
    '0110000110',
    '1100001100',
  ],
  [
    '0001111000',
    '0011AA1100',
    '0011AA1100',
    '0001111000',
    '0111111110',
    '0111111110',
    '1011111101',
    '1001111001',
    '0011001100',
    '0110000110',
  ],
]

// Virus: a spiked capsid whose spikes rotate around the core.
const VIRUS_FRAMES = [
  [
    '0001001000',
    '0010110100',
    '0101111010',
    '1011111101',
    '0111AA1110',
    '0111AA1110',
    '1011111101',
    '0101111010',
    '0010110100',
    '0001001000',
  ],
  [
    '0000110000',
    '0011111100',
    '0111111110',
    '1111111111',
    '1111AA1111',
    '1111AA1111',
    '1111111111',
    '0111111110',
    '0011111100',
    '0000110000',
  ],
  [
    '0010000100',
    '0101111010',
    '0011111100',
    '0111111110',
    '1111AA1111',
    '1111AA1111',
    '0111111110',
    '0011111100',
    '0101111010',
    '0010000100',
  ],
  [
    '0001001000',
    '0010110100',
    '1101111011',
    '0111111110',
    '0111AA1110',
    '0111AA1110',
    '0111111110',
    '1101111011',
    '0010110100',
    '0001001000',
  ],
]

// System glitch: a monitor whose scanlines tear sideways.
const SYSTEM_GLITCH_FRAMES = [
  [
    '1111111111',
    '1000000001',
    '1011111101',
    '1010A0A101',
    '1011111101',
    '1000000001',
    '1111111111',
    '0001111000',
    '0001111000',
    '0111111110',
  ],
  [
    '1111111111',
    '1000000001',
    '0110111110',
    '1010A0A101',
    '1111011011',
    '1000000001',
    '1111111111',
    '0001111000',
    '0001111000',
    '0111111110',
  ],
  [
    '1111111111',
    '1000000001',
    '1011111101',
    '0101A0A110',
    '1011111101',
    '1000000001',
    '1111111111',
    '0001111000',
    '0001111000',
    '0111111110',
  ],
  [
    '1111111111',
    '1000000001',
    '1101111011',
    '1010A0A101',
    '0111110111',
    '1000000001',
    '1111111111',
    '0001111000',
    '0001111000',
    '0111111110',
  ],
]

// Hardware fault: a chip with pins that spark on and off.
const HARDWARE_FAULT_FRAMES = [
  [
    '0010010100',
    '0111111110',
    '1100000011',
    '1101111011',
    '110A00A011',
    '1101111011',
    '1100000011',
    '0111111110',
    '0010010100',
  ],
  [
    '0100101000',
    '0111111110',
    '1100000011',
    '1101111011',
    '110A00A011',
    '1101111011',
    '1100000011',
    '0111111110',
    '0100101000',
  ],
  [
    '0010010100',
    '0111111110',
    '1100000011',
    '1101111011',
    '1100AA0011',
    '1101111011',
    '1100000011',
    '0111111110',
    '0010010100',
  ],
  [
    '0100101000',
    '0111111110',
    '1100000011',
    '1101111011',
    '110A00A011',
    '1101111011',
    '1100000011',
    '0111111110',
    '0001001010',
  ],
]

// Each boss bobs so its body crosses the runner's firing line at the bottom of the arc,
// otherwise a standing shot could never connect.
const BOSS_VARIANTS: BossVariant[] = [
  {
    label: 'HACKER',
    frames: HACKER_FRAMES,
    // Deep phosphor green with a brighter green visor glow — old CRT terminal look.
    color: '#14532d',
    accent: '#22c55e',
    hp: 3,
    spawnY: 148,
    targetX: 470,
    approachSpeed: 2.4,
    bobRate: 40,
    bobAmplitude: 16,
    fireInterval: 160,
    attack: 'burst',
  },
  {
    label: 'HUMAN ERROR',
    frames: HUMAN_ERROR_FRAMES,
    color: '#f59e0b',
    hp: 3,
    spawnY: 150,
    targetX: 500,
    approachSpeed: 2.2,
    bobRate: 30,
    bobAmplitude: 12,
    fireInterval: 140,
    attack: 'drift',
  },
  {
    label: 'VIRUS',
    frames: VIRUS_FRAMES,
    color: '#16a34a',
    hp: 3,
    spawnY: 140,
    targetX: 450,
    approachSpeed: 2.6,
    bobRate: 26,
    bobAmplitude: 16,
    fireInterval: 150,
    attack: 'split',
  },
  {
    label: 'SYSTEM GLITCH',
    frames: SYSTEM_GLITCH_FRAMES,
    color: '#e11d48',
    hp: 3,
    spawnY: 146,
    targetX: 480,
    approachSpeed: 2.4,
    bobRate: 22,
    bobAmplitude: 14,
    fireInterval: 125,
    attack: 'flicker',
  },
  {
    label: 'HARDWARE FAULT',
    frames: HARDWARE_FAULT_FRAMES,
    color: '#0891b2',
    hp: 4,
    spawnY: 152,
    targetX: 505,
    approachSpeed: 2,
    bobRate: 46,
    bobAmplitude: 10,
    fireInterval: 150,
    attack: 'slab',
  },
]

const CLOUD_SPRITE = ['000111000', '011111110', '111111111', '011111110']

// Background palettes the run cycles through as score climbs. resolveTheme() holds each
// steady, then cross-fades into the next only in its final stretch, so the shift reads as
// gradual rather than a hard cut when the milestone hits.
type Theme = {
  skyTop: string
  skyBottom: string
  hill: string
  cloud: string
  ground: string
  pebble: string
  tower: string
  night: boolean
}

const THEMES: Theme[] = [
  {
    skyTop: '#eef2ff',
    skyBottom: '#ffffff',
    hill: '#e5e5e5',
    cloud: '#f4f4f5',
    ground: '#d4d4d4',
    pebble: '#a3a3a3',
    tower: '#e0e0e5',
    night: false,
  },
  {
    skyTop: '#fed7aa',
    skyBottom: '#fff1e6',
    hill: '#fdba74',
    cloud: '#fef3c7',
    ground: '#fb923c',
    pebble: '#c2620c',
    tower: '#fca85c',
    night: false,
  },
  {
    skyTop: '#1e1b4b',
    skyBottom: '#312e81',
    hill: '#4338ca',
    cloud: '#4f46e5',
    ground: '#818cf8',
    pebble: '#a5b4fc',
    tower: '#3730a3',
    night: true,
  },
  {
    skyTop: '#052e2b',
    skyBottom: '#0f766e',
    hill: '#115e59',
    cloud: '#2dd4bf',
    ground: '#5eead4',
    pebble: '#99f6e4',
    tower: '#134e4a',
    night: true,
  },
]

const THEME_STEP = 400

// Autopilot planner: how far ahead a committed action may be deferred, and which actions
// are worth deferring. Deferral has to cover a full jump arc (~38 frames) plus slack.
const AI_MAX_DELAY = 36
const AI_ACTIONS = ['duck', 'jump'] as const

const hexToRgb = (hex: string): [number, number, number] => {
  const n = parseInt(hex.slice(1), 16)
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255]
}

const lerpColor = (a: string, b: string, t: number) => {
  const [ar, ag, ab] = hexToRgb(a)
  const [br, bg, bb] = hexToRgb(b)
  return `rgb(${Math.round(ar + (br - ar) * t)}, ${Math.round(ag + (bg - ag) * t)}, ${Math.round(ab + (bb - ab) * t)})`
}

function resolveTheme(score: number) {
  const idx = Math.floor(score / THEME_STEP) % THEMES.length
  const a = THEMES[idx]
  const b = THEMES[(idx + 1) % THEMES.length]
  const progress = (score % THEME_STEP) / THEME_STEP
  const t = Math.max(0, (progress - 0.7) / 0.3)
  return {
    skyTop: lerpColor(a.skyTop, b.skyTop, t),
    skyBottom: lerpColor(a.skyBottom, b.skyBottom, t),
    hill: lerpColor(a.hill, b.hill, t),
    cloud: lerpColor(a.cloud, b.cloud, t),
    ground: lerpColor(a.ground, b.ground, t),
    pebble: lerpColor(a.pebble, b.pebble, t),
    tower: lerpColor(a.tower, b.tower, t),
    nightLevel: (a.night ? 1 : 0) * (1 - t) + (b.night ? 1 : 0) * t,
  }
}

type ObstacleKind = 'bug' | 'error' | 'malware'
type Obstacle = { x: number; y: number; sprite: string[]; kind: ObstacleKind; hp: number }
type Bolt = { x: number; y: number }
type ShotBehavior = 'straight' | 'drift' | 'split' | 'flicker'
type Shot = {
  x: number
  y: number
  vx: number
  vy: number
  size: number
  t: number
  behavior: ShotBehavior
  split: boolean
}
type Particle = { x: number; y: number; vx: number; vy: number; life: number }
// Every boss owns a different attack, so learning one fight never solves the next.
type BossAttack = 'burst' | 'drift' | 'split' | 'flicker' | 'slab'
type BossVariant = {
  label: string
  frames: string[][]
  color: string
  accent?: string
  hp: number
  spawnY: number
  targetX: number
  approachSpeed: number
  bobRate: number
  bobAmplitude: number
  fireInterval: number
  attack: BossAttack
}
type Boss = {
  x: number
  y: number
  hp: number
  maxHp: number
  t: number
  cooldown: number
  volley: number
  burst: number
  burstHigh: boolean
  animation: number
  sprite: string[]
  variant: BossVariant
}
type Scenery = {
  x: number
  kind: 'cloud' | 'hill' | 'pebble' | 'tower' | 'star' | 'bird'
  y: number
  size: number
}

export default function RuvyxaRunner() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null)

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    let raf = 0
    let running = true
    let started = false
    let gameOver = false
    let paused = false
    let score = 0
    let best = 0
    let speed = 4.5
    let frame = 0
    let nextSpawnIn = 70
    let ammo = MAX_AMMO
    let ammoTick = 0
    let ducking = false
    let nextBossAt = 250
    let autoPlay = false
    let aiShotCooldown = 0
    let aiRestartTimer = 0
    // Persists across restarts on purpose — that is what makes the autopilot improve
    // run over run instead of repeating the same death.
    let aiCaution = 0
    let aiLastDeathScore = 0

    const runner = { x: 48, y: GROUND_Y - 8 * PIXEL, vy: 0, onGround: true }
    let obstacles: Obstacle[] = []
    let bolts: Bolt[] = []
    let shots: Shot[] = []
    let particles: Particle[] = []
    let boss: Boss | null = null
    let scenery: Scenery[] = []

    function seedScenery() {
      scenery = []
      for (let i = 0; i < 3; i++) {
        scenery.push({
          x: 120 + i * 260,
          kind: 'cloud',
          y: 26 + ((i * 17) % 30),
          size: 2 + (i % 2),
        })
      }
      for (let i = 0; i < 4; i++) {
        scenery.push({ x: 80 + i * 210, kind: 'hill', y: 0, size: 26 + ((i * 11) % 22) })
      }
      for (let i = 0; i < 10; i++) {
        scenery.push({ x: i * 78, kind: 'pebble', y: 0, size: 1 + (i % 3) })
      }
      for (let i = 0; i < 5; i++) {
        scenery.push({ x: i * 170, kind: 'tower', y: 0, size: 28 + ((i * 13) % 34) })
      }
      for (let i = 0; i < 14; i++) {
        scenery.push({
          x: Math.random() * WIDTH,
          kind: 'star',
          y: 8 + Math.random() * 70,
          size: 1 + Math.random() * 1.5,
        })
      }
      for (let i = 0; i < 2; i++) {
        scenery.push({ x: 220 + i * 320, kind: 'bird', y: 34 + i * 22, size: 2 })
      }
    }
    seedScenery()

    function reset() {
      runner.y = GROUND_Y - 8 * PIXEL
      runner.vy = 0
      runner.onGround = true
      obstacles = []
      bolts = []
      shots = []
      particles = []
      boss = null
      score = 0
      speed = 4.5
      frame = 0
      nextSpawnIn = 70
      ammo = MAX_AMMO
      ammoTick = 0
      ducking = false
      nextBossAt = 250
      gameOver = false
      paused = false
      aiShotCooldown = 0
      aiRestartTimer = 0
      seedScenery()
    }

    function jump() {
      if (!started) {
        started = true
        return
      }
      if (gameOver) {
        reset()
        return
      }
      if (paused) return
      if (runner.onGround) {
        runner.vy = JUMP_VELOCITY
        runner.onGround = false
      }
    }

    function shoot() {
      if (!started || gameOver || paused || ammo <= 0) return
      ammo--
      bolts.push({ x: runner.x + 8 * PIXEL, y: runner.y + (ducking ? 6 : 10) })
    }

    function burst(x: number, y: number, n: number) {
      for (let i = 0; i < n; i++) {
        particles.push({
          x,
          y,
          vx: (Math.random() - 0.5) * 5,
          vy: -Math.random() * 3.5,
          life: 18 + Math.random() * 12,
        })
      }
    }

    // Standing clears a high shot only by crouching; a low shot has to be jumped.
    const HIGH_LANE = GROUND_Y - 26
    const LOW_LANE = GROUND_Y - 14

    function makeShot(
      x: number,
      y: number,
      opts: { vx?: number; vy?: number; size?: number; behavior?: ShotBehavior } = {},
    ): Shot {
      return {
        x,
        y,
        vx: opts.vx ?? 5.5,
        vy: opts.vy ?? 0,
        size: opts.size ?? 9,
        t: 0,
        behavior: opts.behavior ?? 'straight',
        split: false,
      }
    }

    function fireBoss(b: Boss) {
      const v = b.variant
      if (v.attack === 'burst') {
        // Two rounds down one lane with a readable gap, then a long reload.
        if (b.burst <= 0) {
          b.burst = 2
          b.burstHigh = b.volley % 2 === 0
        }
        shots.push(makeShot(b.x, b.burstHigh ? HIGH_LANE : LOW_LANE, { vx: 5.5 }))
        b.burst--
        b.cooldown = b.burst > 0 ? 24 : v.fireInterval
      } else if (v.attack === 'drift') {
        // Lobbed high and sinking — it settles into the standing lane, so it must be ducked.
        shots.push(makeShot(b.x, GROUND_Y - 64, { vx: 4.6, vy: 0.42, behavior: 'drift' }))
        b.cooldown = v.fireInterval
      } else if (v.attack === 'split') {
        // One round that clones itself midway: duck the leader, then jump the trailer.
        shots.push(makeShot(b.x, HIGH_LANE, { vx: 5, behavior: 'split' }))
        b.cooldown = v.fireInterval
      } else if (v.attack === 'flicker') {
        // Jumps between lanes while travelling, then locks in with room left to react.
        shots.push(makeShot(b.x, LOW_LANE, { vx: 5, behavior: 'flicker' }))
        b.cooldown = v.fireInterval
      } else {
        // Slow oversized wall — too tall to crouch under, so the only answer is a jump.
        shots.push(makeShot(b.x, GROUND_Y - 22, { vx: 3, size: 18 }))
        b.cooldown = v.fireInterval
      }
      b.volley++
    }

    function togglePause() {
      if (!started || gameOver) return
      paused = !paused
      ducking = false
    }

    function drawSprite(
      sprite: string[],
      x: number,
      y: number,
      scale = PIXEL,
      color = INK,
      accentColor = ACCENT,
    ) {
      for (let row = 0; row < sprite.length; row++) {
        const line = sprite[row]
        for (let col = 0; col < line.length; col++) {
          const cell = line[col]
          if (cell === '0') continue
          ctx!.fillStyle = cell === 'A' ? accentColor : cell === 'K' ? INK : color
          ctx!.fillRect(x + col * scale, y + row * scale, scale, scale)
        }
      }
    }

    // A 1px rim in a background-contrasting color, drawn behind a sprite so it never
    // blends into a same-hue theme (e.g. a purple runner over a purple night sky).
    function drawOutline(sprite: string[], x: number, y: number, scale: number, color: string) {
      ctx!.fillStyle = color
      const offsets: Array<[number, number]> = [
        [-1, 0],
        [1, 0],
        [0, -1],
        [0, 1],
      ]
      for (let row = 0; row < sprite.length; row++) {
        const line = sprite[row]
        for (let col = 0; col < line.length; col++) {
          if (line[col] === '0') continue
          for (const [dx, dy] of offsets) {
            ctx!.fillRect(x + col * scale + dx, y + row * scale + dy, scale, scale)
          }
        }
      }
    }

    const sprH = (s: string[]) => s.length * PIXEL
    const sprW = (s: string[]) => s[0].length * PIXEL
    const pick = <T,>(list: readonly T[]): T => list[Math.floor(Math.random() * list.length)]

    type Box = { x: number; y: number; w: number; h: number }
    const overlap = (a: Box, b: Box) =>
      a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y

    function runnerBox(): Box {
      const sprite = ducking && runner.onGround ? RUNNER_DUCK : RUNNER_SPRITE
      const h = sprH(sprite)
      return { x: runner.x + 3, y: GROUND_Y - h + 3, w: sprW(sprite) - 6, h: h - 6 }
    }

    function airborneBox(): Box {
      return { x: runner.x + 3, y: runner.y + 3, w: 8 * PIXEL - 6, h: 8 * PIXEL - 6 }
    }

    function activeRunnerBox(): Box {
      return runner.onGround ? runnerBox() : airborneBox()
    }

    function obstacleBox(o: Obstacle): Box {
      return { x: o.x + 2, y: o.y + 2, w: sprW(o.sprite) - 4, h: sprH(o.sprite) - 4 }
    }

    function spawnObstacle() {
      const roll = Math.random()
      if (roll < 0.55) {
        const sprite = pick(BUG_SPRITES)
        obstacles.push({ x: WIDTH + 10, y: GROUND_Y - sprH(sprite), sprite, kind: 'bug', hp: 1 })
      } else if (roll < 0.8) {
        const sprite = pick(ERROR_SPRITES)
        obstacles.push({ x: WIDTH + 10, y: GROUND_Y - 36, sprite, kind: 'error', hp: 1 })
      } else {
        const sprite = pick(MALWARE_SPRITES)
        obstacles.push({
          x: WIDTH + 10,
          y: GROUND_Y - sprH(sprite),
          sprite,
          kind: 'malware',
          hp: 2,
        })
      }
    }

    function drawScenery(theme: ReturnType<typeof resolveTheme>) {
      // Drawn back-to-front by kind (not array order) so depth stays correct regardless
      // of spawn sequence: sky sparkle, then clouds, skyline, hills, birds, ground grit.
      for (const s of scenery) {
        if (s.kind !== 'star') continue
        if (theme.nightLevel < 0.05) continue
        const twinkle = 0.4 + 0.6 * Math.abs(Math.sin(frame / 20 + s.x))
        ctx!.fillStyle = `rgba(255, 255, 255, ${(theme.nightLevel * twinkle).toFixed(2)})`
        ctx!.fillRect(s.x, s.y, s.size, s.size)
      }
      for (const s of scenery) {
        if (s.kind === 'cloud') drawSprite(CLOUD_SPRITE, s.x, s.y, s.size, theme.cloud)
      }
      for (const s of scenery) {
        if (s.kind !== 'tower') continue
        ctx!.fillStyle = theme.tower
        const w = Math.max(10, Math.floor(s.size * 0.55))
        ctx!.fillRect(s.x, GROUND_Y - s.size, w, s.size)
      }
      for (const s of scenery) {
        if (s.kind !== 'hill') continue
        ctx!.fillStyle = theme.hill
        const steps = 5
        const stepW = Math.max(4, Math.floor(s.size / steps))
        for (let i = 0; i < steps; i++) {
          const h = Math.round((s.size * (i + 1)) / steps)
          ctx!.fillRect(s.x + i * stepW, GROUND_Y - h, stepW, h)
          ctx!.fillRect(s.x + (steps * 2 - i - 1) * stepW, GROUND_Y - h, stepW, h)
        }
      }
      for (const s of scenery) {
        if (s.kind !== 'bird') continue
        ctx!.fillStyle = theme.hill
        ctx!.fillRect(s.x, s.y, s.size, s.size)
        ctx!.fillRect(s.x - s.size * 2, s.y + s.size, s.size, s.size)
        ctx!.fillRect(s.x + s.size * 2, s.y + s.size, s.size, s.size)
      }
      for (const s of scenery) {
        if (s.kind !== 'pebble') continue
        ctx!.fillStyle = theme.pebble
        ctx!.fillRect(s.x, GROUND_Y + 8, s.size * PIXEL, PIXEL)
      }
    }

    function endGame() {
      gameOver = true
      best = Math.max(best, score)
      burst(runner.x + 12, runner.y + 12, 14)
      if (autoPlay) {
        aiRestartTimer = 45
        aiCaution = Math.min(4, aiCaution + 1)
        aiLastDeathScore = 0
      }
    }

    // A threat snapshot the planner can fast-forward without touching live game state.
    type SimThreat = {
      x: number
      y: number
      w: number
      h: number
      vx: number
      vy: number
      t: number
      size: number
      behavior: ShotBehavior
      split: boolean
    }

    function snapshotThreats(): SimThreat[] {
      const list: SimThreat[] = []
      for (const o of obstacles) {
        if (o.hp <= 0) continue
        list.push({
          x: o.x + 2,
          y: o.y + 2,
          w: sprW(o.sprite) - 4,
          h: sprH(o.sprite) - 4,
          vx: speed,
          vy: 0,
          t: 0,
          size: 0,
          behavior: 'straight',
          split: true,
        })
      }
      for (const s of shots) {
        list.push({
          x: s.x,
          y: s.y,
          w: s.size + 1,
          h: s.size + 1,
          vx: s.vx,
          vy: s.vy,
          t: s.t,
          size: s.size,
          behavior: s.behavior,
          split: s.split,
        })
      }
      return list
    }

    // Mirrors the real shot/obstacle update exactly, including the split clone, so the
    // planner never mispredicts where a threat will actually be.
    function advanceThreats(list: SimThreat[]): SimThreat[] {
      const born: SimThreat[] = []
      for (const th of list) {
        th.t++
        th.x -= th.vx
        if (th.behavior === 'drift') {
          th.y = Math.min(th.y + th.vy, GROUND_Y - 22)
        } else if (th.behavior === 'flicker') {
          if (th.x > 220 && th.t % 26 === 0) th.y = th.y < GROUND_Y - 20 ? LOW_LANE : HIGH_LANE
        } else if (th.behavior === 'split' && !th.split && th.x < 220) {
          th.split = true
          born.push({
            x: th.x + 100,
            y: LOW_LANE,
            w: th.size + 1,
            h: th.size + 1,
            vx: th.vx,
            vy: 0,
            t: 0,
            size: th.size,
            behavior: 'straight',
            split: true,
          })
        }
      }
      return born.length ? list.concat(born) : list
    }

    type AiAction = 'none' | 'jump' | 'duck'

    function cloneThreats(base: SimThreat[]): SimThreat[] {
      const out: SimThreat[] = new Array(base.length)
      for (let i = 0; i < base.length; i++) {
        const t = base[i]
        out[i] = {
          x: t.x,
          y: t.y,
          w: t.w,
          h: t.h,
          vx: t.vx,
          vy: t.vy,
          t: t.t,
          size: t.size,
          behavior: t.behavior,
          split: t.split,
        }
      }
      return out
    }

    // Evaluates a plan — "wait `delay` frames, then commit to `action` and hold it" — by
    // rolling the runner's real physics and hitboxes forward, returning frames survived.
    //
    // The delay is the whole point. A planner that can only hold an action from right now
    // cannot express "wait, THEN jump", so it takes the first jump that looks marginally
    // better and lands straight onto a tall obstacle. Searching the delay lets it hold
    // position and jump on the frame that actually clears.
    function simulatePlan(
      base: SimThreat[],
      action: AiAction,
      delay: number,
      horizon: number,
      pad: number,
    ): number {
      if (action === 'jump' && delay === 0 && !runner.onGround) return -1
      let ry = runner.y
      let rvy = runner.vy
      let onGround = runner.onGround
      let jumped = false
      let threats = cloneThreats(base)

      for (let f = 0; f < horizon; f++) {
        const active = f >= delay
        if (action === 'jump' && active && !jumped && onGround) {
          rvy = JUMP_VELOCITY
          onGround = false
          jumped = true
        }
        const duckHeld = action === 'duck' && active
        rvy += GRAVITY
        if (duckHeld && !onGround) rvy += 0.7
        ry += rvy
        if (ry >= GROUND_Y - 8 * PIXEL) {
          ry = GROUND_Y - 8 * PIXEL
          rvy = 0
          onGround = true
        }
        threats = advanceThreats(threats)

        let box: Box
        if (onGround) {
          const sprite = duckHeld ? RUNNER_DUCK : RUNNER_SPRITE
          const h = sprH(sprite)
          box = { x: runner.x + 3, y: GROUND_Y - h + 3, w: sprW(sprite) - 6, h: h - 6 }
        } else {
          box = { x: runner.x + 3, y: ry + 3, w: 8 * PIXEL - 6, h: 8 * PIXEL - 6 }
        }
        const padded = {
          x: box.x - pad,
          y: box.y - pad,
          w: box.w + pad * 2,
          h: box.h + pad * 2,
        }
        for (const th of threats) {
          if (th.x > WIDTH) continue
          if (overlap(padded, th)) return f
        }
      }
      return horizon
    }

    // Would a bolt fired right now actually connect? Keeps the AI from dumping ammo into
    // empty track and arriving at a boss with nothing loaded.
    function boltWouldHit(): boolean {
      const boltY = runner.y + (ducking ? 6 : 10)
      let bx = runner.x + 8 * PIXEL
      const targets = obstacles
        .filter((o) => o.hp > 0)
        .map((o) => ({
          x: o.x + 2,
          y: o.y + 2,
          w: sprW(o.sprite) - 4,
          h: sprH(o.sprite) - 4,
        }))
      const b = boss
      let bossX = b ? b.x : 0
      let bossT = b ? b.t : 0

      for (let f = 0; f < 60; f++) {
        bx += 15
        if (bx > WIDTH + 20) break
        for (const t of targets) t.x -= speed
        const bolt = { x: bx, y: boltY, w: 10, h: 4 }
        for (const t of targets) {
          if (overlap(bolt, t)) return true
        }
        if (b && b.hp > 0) {
          bossT++
          if (bossX > b.variant.targetX) bossX -= b.variant.approachSpeed
          const bossY =
            b.variant.spawnY + Math.sin(bossT / b.variant.bobRate) * b.variant.bobAmplitude
          const bossBox = {
            x: bossX,
            y: bossY,
            w: sprW(b.sprite),
            h: sprH(b.sprite),
          }
          if (overlap(bolt, bossBox)) return true
        }
      }
      return false
    }

    // Alt+T autopilot. Each frame it plans by simulation rather than by rule-of-thumb
    // timings, then adapts its safety margin from how the previous runs actually ended.
    function autoPilot() {
      if (!autoPlay || paused) return
      if (!started) {
        jump()
        return
      }
      if (gameOver) {
        if (aiRestartTimer > 0) {
          aiRestartTimer--
          return
        }
        jump()
        return
      }

      // Caution is the learned part: every death widens the margin and lookahead, and a
      // long clean stretch narrows them again, so the AI settles on the least-twitchy
      // margin that still survives the speed it is currently running at.
      if (score - aiLastDeathScore > 500 && aiCaution > 0) {
        aiCaution = Math.max(0, aiCaution - 1)
        aiLastDeathScore = score
      }
      const pad = 1 + aiCaution
      const horizon = 52 + aiCaution * 6
      const base = snapshotThreats()

      // Searches every plan and keeps the best, but only ever executes its FIRST frame —
      // next frame it re-plans from scratch against whatever actually happened.
      function planBest(padding: number) {
        let bestAction: AiAction = 'none'
        let bestScore = simulatePlan(base, 'none', 0, horizon, padding)
        let bestPref = 0
        for (let delay = 0; delay <= AI_MAX_DELAY; delay++) {
          for (const action of AI_ACTIONS) {
            const score = simulatePlan(base, action, delay, horizon, padding)
            if (score < 0) continue
            const immediate: AiAction = delay === 0 ? action : 'none'
            const pref = immediate === 'none' ? 0 : immediate === 'duck' ? 1 : 2
            if (score > bestScore || (score === bestScore && pref < bestPref)) {
              bestScore = score
              bestAction = immediate
              bestPref = pref
            }
            // A clean run that needs nothing this frame is the ideal outcome; stop early.
            if (bestScore >= horizon && bestPref === 0) {
              return { action: bestAction, score: bestScore }
            }
          }
        }
        return { action: bestAction, score: bestScore }
      }

      let best = planBest(pad)
      // Nothing survives the padded margin — retry unpadded so a tight-but-real gap still
      // gets taken instead of the AI freezing and eating the hit.
      if (best.score < horizon && pad > 0) {
        const tight = planBest(0)
        if (tight.score > best.score) best = tight
      }

      ducking = best.action === 'duck'
      if (best.action === 'jump' && runner.onGround) jump()

      if (aiShotCooldown > 0) aiShotCooldown--
      // Only shoot from a stable pose; firing mid-dodge is what wastes the clip.
      if (ammo > 0 && aiShotCooldown <= 0 && best.action === 'none' && boltWouldHit()) {
        shoot()
        aiShotCooldown = 8
      }
    }

    function step() {
      if (!running) return
      if (!paused) frame++
      autoPilot()

      ctx!.clearRect(0, 0, WIDTH, HEIGHT)
      const theme = resolveTheme(score)
      const sky = ctx!.createLinearGradient(0, 0, 0, HEIGHT)
      sky.addColorStop(0, theme.skyTop)
      sky.addColorStop(1, theme.skyBottom)
      ctx!.fillStyle = sky
      ctx!.fillRect(0, 0, WIDTH, HEIGHT)
      drawScenery(theme)
      // Tracks the sky's light/dark balance continuously, so the runner, boss, and
      // shots stay readable through a theme cross-fade instead of flipping at a hard cutoff.
      const outline = lerpColor('#171717', '#fafafa', theme.nightLevel)

      ctx!.strokeStyle = theme.ground
      ctx!.lineWidth = 2
      ctx!.beginPath()
      ctx!.moveTo(0, GROUND_Y + 2)
      ctx!.lineTo(WIDTH, GROUND_Y + 2)
      ctx!.stroke()

      if (started && !gameOver && !paused) {
        for (const s of scenery) {
          const factor =
            s.kind === 'star'
              ? 0.04
              : s.kind === 'tower'
                ? 0.12
                : s.kind === 'cloud'
                  ? 0.18
                  : s.kind === 'hill'
                    ? 0.35
                    : s.kind === 'bird'
                      ? 0.6
                      : 1
          s.x -= speed * factor
          if (s.x < -80) s.x = WIDTH + Math.random() * 120
        }

        runner.vy += GRAVITY
        if (ducking && !runner.onGround) runner.vy += 0.7
        runner.y += runner.vy
        if (runner.y >= GROUND_Y - 8 * PIXEL) {
          runner.y = GROUND_Y - 8 * PIXEL
          runner.vy = 0
          runner.onGround = true
        }

        ammoTick++
        // Refill faster during a boss fight so the player is never stuck empty.
        if (ammoTick >= (boss ? 40 : AMMO_REGEN)) {
          ammoTick = 0
          ammo = Math.min(MAX_AMMO, ammo + 1)
        }

        if (!boss && score >= nextBossAt) {
          const variant = pick(BOSS_VARIANTS)
          boss = {
            x: WIDTH + 40,
            y: variant.spawnY,
            hp: variant.hp,
            maxHp: variant.hp,
            t: 0,
            cooldown: variant.fireInterval,
            volley: 0,
            burst: 0,
            burstHigh: false,
            animation: 0,
            sprite: variant.frames[0],
            variant,
          }
        }

        if (!boss) {
          nextSpawnIn--
          if (nextSpawnIn <= 0) {
            spawnObstacle()
            nextSpawnIn = 60 + Math.floor(Math.random() * 45)
          }
        }

        for (const o of obstacles) o.x -= speed
        obstacles = obstacles.filter((o) => o.x > -40)

        // boss behaviour
        if (boss) {
          const v = boss.variant
          boss.t++
          // Four-frame loop, advanced on the fixed step so every boss idles at the same tempo.
          boss.animation = (boss.animation + 0.12) % v.frames.length
          boss.sprite = v.frames[Math.floor(boss.animation)]
          if (boss.x > v.targetX) boss.x -= v.approachSpeed
          boss.y = v.spawnY + Math.sin(boss.t / v.bobRate) * v.bobAmplitude
          boss.cooldown--
          if (boss.cooldown <= 0) fireBoss(boss)
          const bBox = {
            x: boss.x + 4,
            y: boss.y + 4,
            w: sprW(boss.sprite) - 8,
            h: sprH(boss.sprite) - 8,
          }
          if (overlap(activeRunnerBox(), bBox)) endGame()
        }

        const spawnedShots: Shot[] = []
        for (const s of shots) {
          s.t++
          s.x -= s.vx
          if (s.behavior === 'drift') {
            s.y = Math.min(s.y + s.vy, GROUND_Y - 22)
          } else if (s.behavior === 'flicker') {
            // Stops swapping well before it arrives, so the final lane is always readable.
            if (s.x > 220 && s.t % 26 === 0) s.y = s.y < GROUND_Y - 20 ? LOW_LANE : HIGH_LANE
          } else if (s.behavior === 'split' && !s.split && s.x < 220) {
            s.split = true
            // The clone trails the parent so the pair arrives as two separate reactions.
            spawnedShots.push(makeShot(s.x + 100, LOW_LANE, { vx: s.vx }))
          }
        }
        shots = shots.concat(spawnedShots).filter((s) => s.x > -30)

        for (const b of bolts) b.x += 15
        bolts = bolts.filter((b) => b.x < WIDTH + 20)

        // bolts vs obstacles
        for (const b of bolts) {
          const bBox = { x: b.x, y: b.y, w: 10, h: 4 }
          for (const o of obstacles) {
            if (o.hp > 0 && overlap(bBox, obstacleBox(o))) {
              o.hp--
              b.x = WIDTH + 999
              burst(o.x + sprW(o.sprite) / 2, o.y + sprH(o.sprite) / 2, 8)
              if (o.hp <= 0) score += 25
            }
          }
          if (
            boss &&
            overlap(bBox, { x: boss.x, y: boss.y, w: sprW(boss.sprite), h: sprH(boss.sprite) })
          ) {
            boss.hp--
            b.x = WIDTH + 999
            burst(boss.x + sprW(boss.sprite) / 2, boss.y + sprH(boss.sprite) / 2, 12)
            // Landing a hit buys a solid reprieve from return fire.
            boss.cooldown = Math.max(boss.cooldown, 60)
            boss.burst = 0
            if (boss.hp <= 0) {
              score += 150
              burst(boss.x + sprW(boss.sprite) / 2, boss.y + sprH(boss.sprite) / 2, 24)
              boss = null
              nextBossAt = score + 350
              shots = []
            }
          }
        }
        bolts = bolts.filter((b) => b.x < WIDTH + 20)
        obstacles = obstacles.filter((o) => o.hp > 0)

        // collisions against the runner
        const rBox = activeRunnerBox()
        for (const o of obstacles) {
          if (overlap(rBox, obstacleBox(o))) endGame()
        }
        for (const s of shots) {
          if (overlap(rBox, { x: s.x, y: s.y, w: s.size + 1, h: s.size + 1 })) endGame()
        }

        if (frame % 6 === 0) score++
        if (frame % 260 === 0) speed = Math.min(speed + 0.35, 10)
      }

      // particles
      if (!paused) {
        for (const p of particles) {
          p.x += p.vx
          p.y += p.vy
          p.vy += 0.2
          p.life--
        }
        particles = particles.filter((p) => p.life > 0)
      }
      ctx!.fillStyle = MUTED
      for (const p of particles) ctx!.fillRect(p.x, p.y, PIXEL, PIXEL)

      // entities
      for (const o of obstacles) drawSprite(o.sprite, o.x, o.y)
      if (boss) {
        drawOutline(boss.sprite, boss.x, boss.y, PIXEL, outline)
        drawSprite(boss.sprite, boss.x, boss.y, PIXEL, boss.variant.color, boss.variant.accent)
      }

      ctx!.fillStyle = ACCENT
      for (const b of bolts) ctx!.fillRect(b.x, b.y, 10, 4)
      for (const s of shots) {
        const core = Math.max(3, Math.round(s.size / 3))
        ctx!.fillStyle = outline
        ctx!.fillRect(s.x - 1, s.y - 1, s.size + 2, s.size + 2)
        ctx!.fillStyle = INK
        ctx!.fillRect(s.x, s.y, s.size, s.size)
        ctx!.fillStyle = ACCENT
        ctx!.fillRect(s.x + core, s.y + core, core, core)
      }

      const duckNow = ducking && runner.onGround
      const gaitFrame = Math.floor(frame / 8) % RUNNER_FRAMES.length
      const runSprite = duckNow
        ? RUNNER_DUCK
        : !started || !runner.onGround
          ? RUNNER_SPRITE
          : RUNNER_FRAMES[gaitFrame]
      const gaitBob =
        !duckNow && started && runner.onGround && (gaitFrame === 1 || gaitFrame === 3) ? -1 : 0
      const runnerDrawY = (duckNow ? GROUND_Y - sprH(RUNNER_DUCK) : runner.y) + gaitBob
      drawOutline(runSprite, runner.x, runnerDrawY, PIXEL, outline)
      drawSprite(runSprite, runner.x, runnerDrawY, PIXEL, SPRITE_COLOR)

      // HUD
      ctx!.fillStyle = '#525252'
      ctx!.font = "13px 'SFMono-Regular', Consolas, monospace"
      ctx!.textBaseline = 'top'
      ctx!.textAlign = 'right'
      ctx!.fillText(`SCORE ${String(score).padStart(5, '0')}`, WIDTH - 20, 14)
      if (best > 0) ctx!.fillText(`BEST ${String(best).padStart(5, '0')}`, WIDTH - 20, 32)
      if (autoPlay) {
        ctx!.fillStyle = ACCENT
        ctx!.fillText(`AUTO · CAUTION ${aiCaution}`, WIDTH - 20, best > 0 ? 50 : 32)
      }

      ctx!.textAlign = 'left'
      ctx!.fillText('FIX', 20, 14)
      for (let i = 0; i < MAX_AMMO; i++) {
        ctx!.fillStyle = i < ammo ? ACCENT : FAINT
        ctx!.fillRect(52 + i * 12, 15, 8, 10)
      }

      if (boss) {
        ctx!.fillStyle = '#525252'
        ctx!.fillText(boss.variant.label, 20, 36)
        // Boss names vary in length, so measure rather than assume a fixed bar offset.
        const barX = 20 + Math.ceil(ctx!.measureText(boss.variant.label).width) + 12
        for (let i = 0; i < boss.maxHp; i++) {
          ctx!.fillStyle = i < boss.hp ? boss.variant.color : FAINT
          ctx!.fillRect(barX + i * 12, 37, 8, 10)
        }
      }

      if (!started || gameOver) {
        ctx!.fillStyle = INK
        ctx!.font = "15px 'SFMono-Regular', Consolas, monospace"
        ctx!.textAlign = 'center'
        ctx!.fillText(
          gameOver ? 'SYSTEM DOWN — SPACE TO RESTART' : 'PRESS SPACE OR TAP TO PLAY',
          WIDTH / 2,
          86,
        )
        ctx!.fillStyle = '#525252'
        ctx!.font = "12px 'SFMono-Regular', Consolas, monospace"
        ctx!.fillText('SPACE/W JUMP   S DUCK   X/ARROWS SHOOT   ESC PAUSE', WIDTH / 2, 110)
        ctx!.fillText('ALT+T TOGGLE AI AUTOPLAY', WIDTH / 2, 126)
      }
      if (paused && !gameOver) {
        ctx!.fillStyle = 'rgba(255, 255, 255, 0.82)'
        ctx!.fillRect(0, 0, WIDTH, HEIGHT)
        ctx!.fillStyle = INK
        ctx!.font = "18px 'SFMono-Regular', Consolas, monospace"
        ctx!.textAlign = 'center'
        ctx!.fillText('PAUSED', WIDTH / 2, 82)
        ctx!.fillStyle = '#525252'
        ctx!.font = "12px 'SFMono-Regular', Consolas, monospace"
        ctx!.fillText('PRESS ESC TO RESUME', WIDTH / 2, 108)
      }
      ctx!.textAlign = 'left'

      raf = requestAnimationFrame(step)
    }

    const JUMP_KEYS = new Set(['Space', 'ArrowUp', 'KeyW'])
    const DUCK_KEYS = new Set(['ArrowDown', 'KeyS'])
    const PAUSE_KEYS = new Set(['Escape'])
    // No lateral movement in an endless runner, so left/right fire ahead instead of going unused.
    const SHOOT_KEYS = new Set(['KeyX', 'KeyF', 'ArrowLeft', 'ArrowRight', 'KeyA', 'KeyD'])

    function toggleAutoPlay() {
      autoPlay = !autoPlay
      ducking = false
      aiRestartTimer = 0
    }

    function onKeyDown(e: KeyboardEvent) {
      if (e.altKey && e.code === 'KeyT') {
        e.preventDefault()
        if (!e.repeat) toggleAutoPlay()
        return
      }
      if (PAUSE_KEYS.has(e.code)) {
        e.preventDefault()
        togglePause()
        return
      }
      // The AI drives jump/duck/shoot itself; manual input would only fight it.
      if (autoPlay) return
      if (JUMP_KEYS.has(e.code)) {
        e.preventDefault()
        jump()
      } else if (DUCK_KEYS.has(e.code)) {
        e.preventDefault()
        ducking = true
      } else if (SHOOT_KEYS.has(e.code)) {
        e.preventDefault()
        shoot()
      }
    }

    function onKeyUp(e: KeyboardEvent) {
      if (autoPlay) return
      if (DUCK_KEYS.has(e.code)) ducking = false
    }

    function onPointerDown() {
      if (autoPlay) return
      jump()
    }

    window.addEventListener('keydown', onKeyDown)
    window.addEventListener('keyup', onKeyUp)
    canvas.addEventListener('pointerdown', onPointerDown)
    raf = requestAnimationFrame(step)

    return () => {
      running = false
      cancelAnimationFrame(raf)
      window.removeEventListener('keydown', onKeyDown)
      window.removeEventListener('keyup', onKeyUp)
      canvas.removeEventListener('pointerdown', onPointerDown)
    }
  }, [])

  return (
    <div className="runner">
      <canvas
        ref={canvasRef}
        className="runner-canvas"
        width={WIDTH}
        height={HEIGHT}
        role="img"
        aria-label="Endless runner mini-game: jump, duck, shoot, and pause while dodging bugs, errors, and malware and defeating animated bosses. Alt+T toggles an AI autopilot."
      />
    </div>
  )
}
