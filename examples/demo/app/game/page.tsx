import RuvyxaRunner from '../components/ruvyxa-runner'

export default function Game() {
  return (
    <main className="page">
      <p className="eyebrow">Client component</p>
      <h1>Mini-Game</h1>
      <p>
        A canvas-based endless runner rendered by a <code>&apos;use client&apos;</code> component —
        jump, duck, and shoot fixes at bugs, errors, and a hacker boss.
      </p>
      <RuvyxaRunner />
    </main>
  )
}
