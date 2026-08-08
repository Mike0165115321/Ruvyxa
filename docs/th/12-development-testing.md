# Development และ testing

> **เป้าหมายของ tutorial:** ตั้ง contributor loop และเลือก test
> ที่เล็กที่สุดซึ่งพิสูจน์การเปลี่ยนแปลงได้ **เริ่มจาก:** boundary map ใน
> [Architecture](11-architecture.md) **Checkpoint:** รัน check ที่แคบที่สุดซึ่งเกี่ยวข้องก่อนเลือก
> repository gate ที่กว้างขึ้น

## การตั้งค่าสำหรับ framework contributor

นี่คือ Rust workspace พร้อม pnpm workspace ติดตั้ง Node version และ pnpm ที่ประกาศไว้ จากนั้นใช้
Rust toolchain ที่เข้ากับ locked workspace

```bash
pnpm install
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --locked -- -D warnings
pnpm -r build
pnpm -r check
pnpm -r test
pnpm format:check
```

สำหรับ broad fixture ให้ใช้คำสั่งที่ repository guide กำหนดไว้ตรงตัว:

```bash
cargo run -p ruvyxa_cli -- check --root examples/demo
cargo run -p ruvyxa_cli -- test:parity --root examples/demo
```

## Test layer

Rust test อยู่กับ crate ที่เกี่ยวข้องและครอบคลุม CLI/graph/bundler/server behavior package test
รันผ่าน Node built-in test runner; package manifest ชี้ไปที่ `tests/packages/**` หรือ package-local
test `@ruvyxa/react` มี test สำหรับ client router `@ruvyxa/testing` ให้ unit test สร้าง
loader/action/cache double และตรวจ call กับ invalidation ได้

```ts
import test from 'node:test'
import assert from 'node:assert/strict'
import { mockAction } from '@ruvyxa/testing'

test('records invalidation', async () => {
  const save = mockAction(({ input, invalidate }) => {
    invalidate('todos')
    return input
  })
  await save({ title: 'Write docs' })
  assert.deepEqual(save.invalidations, ['todos'])
})
```

repository ปัจจุบันมี CI workflow ที่ `.github/workflows/ci.yml` และ `.github/workflows/release.yml`
อย่ากล่าวอ้าง command ของ job รายตัวโดยไม่อ่าน workflow ใน revision ที่กำลังแก้ เพราะ workflow
เปลี่ยนแยกจาก package script ได้

## Definition of done

สำหรับ public framework change ให้ update Rust/TypeScript contract, test, template เมื่อเกี่ยวข้อง
และเอกสารทั้งสองภาษาใน `docs/` รัน test ที่แคบที่สุดระหว่าง iteration แล้วรัน check
ที่กว้างขึ้นด้านบนก่อนส่งมอบ ห้าม commit generated `.ruvyxa/`, `dist/`, `target/`, `node_modules/`
หรือ package smoke directory

**ก่อนหน้า:** [Architecture](11-architecture.md) · **ถัดไป:** [Security](13-security.md)
