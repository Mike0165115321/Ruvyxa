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
const MUTED = '#a3a3a3'
const FAINT = '#e5e5e5'

// 8x8 Ruvyxa runner, with a one-pixel brand accent for the eye.
const RUNNER_SPRITE = [
  '00111100',
  '01111A10',
  '11011011',
  '11111111',
  '01111110',
  '00100100',
  '01000010',
  '01000010',
]

// Alternate leg position so the runner animates while moving.
const RUNNER_SPRITE_ALT = [
  '00111100',
  '01111A10',
  '11011011',
  '11111111',
  '01111110',
  '00100100',
  '00100100',
  '00011000',
]

// Crouched pose — shorter hitbox to slip under flying errors.
const RUNNER_DUCK = ['0011111000', '0111A11100', '1111111110', '0111111100', '0010001000']

// Ground bugs, three sizes.
const BUG_SPRITES = [
  ['011010', '111111', '011110', '111111', '010010', '101101'],
  ['0110110', '1111111', '0111110', '1111111', '0100010', '1011101'],
  ['01101100', '11111110', '01111100', '11111110', '01000100', '10111011'],
]

// Flying error — winged, forces a duck.
const ERROR_SPRITE = ['10011001', '11011011', '01111110', '11111111', '01A11A10', '00100100']

// Tall malware block — forces a jump.
const MALWARE_SPRITE = [
  '011110',
  '111111',
  '1A11A1',
  '111111',
  '010010',
  '111111',
  '101101',
  '010010',
]

// Hooded hacker boss.
const HACKER_SPRITE = [
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
]

const CLOUD_SPRITE = ['000111000', '011111110', '111111111', '011111110']

type ObstacleKind = 'bug' | 'error' | 'malware'
type Obstacle = { x: number; y: number; sprite: string[]; kind: ObstacleKind; hp: number }
type Bolt = { x: number; y: number }
type Shot = { x: number; y: number }
type Particle = { x: number; y: number; vx: number; vy: number; life: number }
type Boss = { x: number; y: number; hp: number; t: number; cooldown: number }
type Scenery = { x: number; kind: 'cloud' | 'hill' | 'pebble'; y: number; size: number }

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
    let score = 0
    let best = 0
    let speed = 4.5
    let frame = 0
    let nextSpawnIn = 70
    let ammo = MAX_AMMO
    let ammoTick = 0
    let ducking = false
    let nextBossAt = 250

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
      if (runner.onGround) {
        runner.vy = JUMP_VELOCITY
        runner.onGround = false
      }
    }

    function shoot() {
      if (!started || gameOver || ammo <= 0) return
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

    function drawSprite(sprite: string[], x: number, y: number, scale = PIXEL, color = INK) {
      for (let row = 0; row < sprite.length; row++) {
        const line = sprite[row]
        for (let col = 0; col < line.length; col++) {
          const cell = line[col]
          if (cell === '0') continue
          ctx!.fillStyle = cell === 'A' ? ACCENT : color
          ctx!.fillRect(x + col * scale, y + row * scale, scale, scale)
        }
      }
    }

    const sprH = (s: string[]) => s.length * PIXEL
    const sprW = (s: string[]) => s[0].length * PIXEL

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
        const sprite = BUG_SPRITES[Math.floor(Math.random() * BUG_SPRITES.length)]
        obstacles.push({
          x: WIDTH + 10,
          y: GROUND_Y - sprH(sprite),
          sprite,
          kind: 'bug',
          hp: 1,
        })
      } else if (roll < 0.8) {
        obstacles.push({
          x: WIDTH + 10,
          y: GROUND_Y - 36,
          sprite: ERROR_SPRITE,
          kind: 'error',
          hp: 1,
        })
      } else {
        obstacles.push({
          x: WIDTH + 10,
          y: GROUND_Y - sprH(MALWARE_SPRITE),
          sprite: MALWARE_SPRITE,
          kind: 'malware',
          hp: 2,
        })
      }
    }

    function drawScenery() {
      for (const s of scenery) {
        if (s.kind === 'cloud') {
          drawSprite(CLOUD_SPRITE, s.x, s.y, s.size, FAINT)
        } else if (s.kind === 'hill') {
          ctx!.fillStyle = FAINT
          const steps = 5
          const stepW = Math.max(4, Math.floor(s.size / steps))
          for (let i = 0; i < steps; i++) {
            const h = Math.round((s.size * (i + 1)) / steps)
            ctx!.fillRect(s.x + i * stepW, GROUND_Y - h, stepW, h)
            ctx!.fillRect(s.x + (steps * 2 - i - 1) * stepW, GROUND_Y - h, stepW, h)
          }
        } else {
          ctx!.fillStyle = MUTED
          ctx!.fillRect(s.x, GROUND_Y + 8, s.size * PIXEL, PIXEL)
        }
      }
    }

    function endGame() {
      gameOver = true
      best = Math.max(best, score)
      burst(runner.x + 12, runner.y + 12, 14)
    }

    function step() {
      if (!running) return
      frame++

      ctx!.clearRect(0, 0, WIDTH, HEIGHT)
      drawScenery()

      ctx!.strokeStyle = '#d4d4d4'
      ctx!.lineWidth = 2
      ctx!.beginPath()
      ctx!.moveTo(0, GROUND_Y + 2)
      ctx!.lineTo(WIDTH, GROUND_Y + 2)
      ctx!.stroke()

      if (started && !gameOver) {
        for (const s of scenery) {
          const factor = s.kind === 'cloud' ? 0.18 : s.kind === 'hill' ? 0.35 : 1
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
          boss = { x: WIDTH + 40, y: 148, hp: 3, t: 0, cooldown: 110 }
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
          boss.t++
          const target = 470
          if (boss.x > target) boss.x -= 3
          // Bob through the runner's firing line so a standing shot can connect.
          boss.y = 148 + Math.sin(boss.t / 34) * 18
          boss.cooldown--
          if (boss.cooldown <= 0) {
            boss.cooldown = 95
            const high = Math.random() < 0.5
            // High shot clears a crouch but not a standing runner; low shot must be jumped.
            shots.push({ x: boss.x, y: high ? GROUND_Y - 26 : GROUND_Y - 14 })
          }
          const bBox = {
            x: boss.x + 4,
            y: boss.y + 4,
            w: sprW(HACKER_SPRITE) - 8,
            h: sprH(HACKER_SPRITE) - 8,
          }
          if (overlap(activeRunnerBox(), bBox)) endGame()
        }

        for (const s of shots) s.x -= 5.5
        shots = shots.filter((s) => s.x > -20)

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
            overlap(bBox, { x: boss.x, y: boss.y, w: sprW(HACKER_SPRITE), h: sprH(HACKER_SPRITE) })
          ) {
            boss.hp--
            b.x = WIDTH + 999
            burst(boss.x + 15, boss.y + 15, 12)
            // Landing a hit buys a short reprieve from return fire.
            boss.cooldown = Math.max(boss.cooldown, 45)
            if (boss.hp <= 0) {
              score += 150
              burst(boss.x + 15, boss.y + 15, 24)
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
          if (overlap(rBox, { x: s.x, y: s.y, w: 10, h: 10 })) endGame()
        }

        if (frame % 6 === 0) score++
        if (frame % 260 === 0) speed = Math.min(speed + 0.35, 10)
      }

      // particles
      for (const p of particles) {
        p.x += p.vx
        p.y += p.vy
        p.vy += 0.2
        p.life--
      }
      particles = particles.filter((p) => p.life > 0)
      ctx!.fillStyle = MUTED
      for (const p of particles) ctx!.fillRect(p.x, p.y, PIXEL, PIXEL)

      // entities
      for (const o of obstacles) drawSprite(o.sprite, o.x, o.y)
      if (boss) drawSprite(HACKER_SPRITE, boss.x, boss.y)

      ctx!.fillStyle = ACCENT
      for (const b of bolts) ctx!.fillRect(b.x, b.y, 10, 4)
      for (const s of shots) {
        ctx!.fillStyle = INK
        ctx!.fillRect(s.x, s.y, 9, 9)
        ctx!.fillStyle = ACCENT
        ctx!.fillRect(s.x + 3, s.y + 3, 3, 3)
      }

      const duckNow = ducking && runner.onGround
      const runSprite = duckNow
        ? RUNNER_DUCK
        : !started || !runner.onGround || Math.floor(frame / 6) % 2 === 0
          ? RUNNER_SPRITE
          : RUNNER_SPRITE_ALT
      drawSprite(runSprite, runner.x, duckNow ? GROUND_Y - sprH(RUNNER_DUCK) : runner.y)

      // HUD
      ctx!.fillStyle = '#525252'
      ctx!.font = "13px 'SFMono-Regular', Consolas, monospace"
      ctx!.textBaseline = 'top'
      ctx!.textAlign = 'right'
      ctx!.fillText(`SCORE ${String(score).padStart(5, '0')}`, WIDTH - 20, 14)
      if (best > 0) ctx!.fillText(`BEST ${String(best).padStart(5, '0')}`, WIDTH - 20, 32)

      ctx!.textAlign = 'left'
      ctx!.fillText('FIX', 20, 14)
      for (let i = 0; i < MAX_AMMO; i++) {
        ctx!.fillStyle = i < ammo ? ACCENT : FAINT
        ctx!.fillRect(52 + i * 12, 15, 8, 10)
      }

      if (boss) {
        ctx!.fillStyle = '#525252'
        ctx!.fillText('HACKER', 20, 36)
        for (let i = 0; i < 3; i++) {
          ctx!.fillStyle = i < boss.hp ? INK : FAINT
          ctx!.fillRect(80 + i * 12, 37, 8, 10)
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
        ctx!.fillText('SPACE/W JUMP   S DUCK   X/ARROWS SHOOT', WIDTH / 2, 110)
      }
      ctx!.textAlign = 'left'

      raf = requestAnimationFrame(step)
    }

    const JUMP_KEYS = new Set(['Space', 'ArrowUp', 'KeyW'])
    const DUCK_KEYS = new Set(['ArrowDown', 'KeyS'])
    // No lateral movement in an endless runner, so left/right fire ahead instead of going unused.
    const SHOOT_KEYS = new Set(['KeyX', 'KeyF', 'ArrowLeft', 'ArrowRight', 'KeyA', 'KeyD'])

    function onKeyDown(e: KeyboardEvent) {
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
      if (DUCK_KEYS.has(e.code)) ducking = false
    }

    function onPointerDown() {
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
        aria-label="Endless runner mini-game: jump, duck, and shoot fixes at bugs, errors, and a hacker boss"
      />
    </div>
  )
}
