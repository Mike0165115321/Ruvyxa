# การค้นพบเว็บไซต์และการปรับภาพให้เหมาะสม

**Source**: `crates/ruvyxa_cli/src/{site_discovery,image_optimizer,image_usage}.rs`

ระบบ build มีส่วนย่อยที่ทำงานร่วมกันสองส่วน: สร้างไฟล์สำหรับ crawler (`robots.txt` และ
`sitemap.xml`) และปรับภาพสาธารณะ PNG/JPEG เป็น WebP พร้อม responsive variants

## `site` ใน `ruvyxa.config.ts`

ตั้งค่าด้วย `site.url`, `site.sitemap` และ `site.robots` โดย `url` ต้องเป็น origin แบบ absolute เช่น
`https://ruvyxa.dev` และห้ามมี path, query หรือ fragment

```ts
import { config } from 'ruvyxa/config'

export default config({
  site: {
    url: 'https://example.com',
    sitemap: true,
    robots: true,
  },
})
```

`sitemap` และ `robots` รับ `false` เพื่อปิด, `true` เพื่อใช้ค่าเริ่มต้น หรือ object สำหรับกำหนด
รายละเอียด เช่น `exclude`, `additionalPaths`, metadata ต่อ URL, crawler rules, sitemap URL และ
`Host` directive

## ลำดับการหา canonical URL

`resolve_site_url()` ใช้ค่าตามลำดับนี้:

1. `config.site.url`
2. `RUVYXA_SITE_URL`
3. `VERCEL_PROJECT_PRODUCTION_URL`
4. `VERCEL_URL` เมื่อ `VERCEL_ENV=production`
5. `URL` เมื่อ `NETLIFY=true`

URL preview จะไม่ถูกใช้เป็น canonical origin ระบบ normalize scheme/host, ตัด trailing slash, ปฏิเสธ
credential และตรวจสอบว่าเป็น HTTP(S) ที่ถูกต้อง

## Sitemap และ robots.txt

`write_discovery_files()` ใช้ route manifest และ output ที่ prerender เพื่อสร้าง `sitemap.xml` เฉพาะ
page ที่ไม่มี dynamic segment เส้นทางที่กำหนดใน `additionalPaths` เพิ่มเข้ามาได้

- จำกัด sitemap ที่ 50,000 URL หรือ 50 MiB ต่อไฟล์ แล้วแบ่งเป็น `sitemap-0.xml`, `sitemap-1.xml` และ
  sitemap index ตามต้องการ
- รองรับ metadata `lastModified`, `changeFrequency`, `priority`, alternates, images และ videos
- encode URL และ escape XML โดย deterministic เพื่อให้ผล build ซ้ำได้
- `public/sitemap.xml` หรือ route `/sitemap.xml` ของโปรเจกต์จะชนะ generation อัตโนมัติ; shard
  ที่ชนชื่อไฟล์จะทำให้ build ล้มเหลวแทนการเขียนทับ

ค่า `robots.txt` เริ่มต้นคืออนุญาต crawler ทั้งหมด แต่ปิด `/__ruvyxa/` และปิด `/api/` เมื่อมี API
routes โปรเจกต์สามารถกำหนด `userAgent`, `allow`, `disallow`, `crawlDelay`, sitemap URLs และ `host`
ได้ `public/robots.txt` หรือ route `/robots.txt` ของโปรเจกต์จะชนะค่าอัตโนมัติเช่นกัน

## Image optimization

`images.optimize` เปิดโดยค่าเริ่มต้น ภาพ PNG/JPEG ใน `public/` ถูกเข้ารหัสเป็น WebP คุณภาพ เริ่มต้น
82 และเก็บไฟล์ต้นฉบับไว้โดยค่าเริ่มต้น ระบบสร้าง variant ที่ 640, 750, 828, 1080, 1200, 1920, 2048
และ 3840 px เฉพาะขนาดที่เล็กกว่าภาพจริง

ขั้นตอนคือ discover → ตรวจชื่อ output ชนกันแบบ case-insensitive → encode → ใช้ cache ที่อิง `blake3`
→ materialize assets → เขียน `.ruvyxa-images.json` ไฟล์ที่อ่านไม่ได้และไฟล์ที่ไม่ใช่ภาพ จะถูก copy
โดยไม่สูญหาย

`images.onDemand` ปิดโดยค่าเริ่มต้น เมื่อเปิด runtime จะรับเฉพาะ request ภาพสาธารณะ same-origin
ที่อยู่ในขอบเขต `maxWidth` (ค่าเริ่มต้น 3840, อนุญาต 16–8192) แล้วตอบ WebP

## การตรวจ raw `<img>`

`scan_raw_image_usage()` แจ้ง warning เมื่อพบ `<img src="/...">` แบบ literal ที่ชี้ไปยังภาพซึ่ง
optimizer ลดขนาดได้อย่างน้อย 8 KiB ผลลัพธ์เป็น warning ไม่ใช่ build error เพราะ raw `<img>`
อาจเป็นการเลือกโดยตั้งใจ ใช้ `<Image>` จาก `@ruvyxa/react` เมื่ออยากใช้ responsive output อัตโนมัติ

## Output ที่สำคัญ

```text
assets/
  robots.txt
  sitemap.xml
  sitemap-0.xml
  logo.png
  logo.webp
  logo-640w.webp
  .ruvyxa-images.json
```

ดู [Configuration](../../guides/th/11-configuration.md) สำหรับ config แบบเต็ม และ
[Markdown, MDX & Images](../../guides/th/09-markdown-mdx-images.md) สำหรับการใช้ `<Image>` ในแอป
