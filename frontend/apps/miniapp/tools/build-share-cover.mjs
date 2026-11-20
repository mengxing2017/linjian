import { deflateSync } from 'node:zlib'
import { writeFileSync } from 'node:fs'

const width = 500
const height = 400
const pixels = Buffer.alloc(height * (width * 3 + 1))
for (let row = 0; row < height; row++) {
  for (let column = 0; column < width; column++) {
    const bubble = column >= 120 && column < 380 && row >= 90 && row < 270
    const tail = row >= 270 && row < 305 && column >= 150 && column < 185 - (row - 270)
    const letter = (column >= 175 && column < 190 && row >= 135 && row < 225)
      || (column >= 175 && column < 225 && row >= 210 && row < 225)
      || (column >= 265 && column < 315 && row >= 135 && row < 150)
      || (column >= 290 && column < 305 && row >= 135 && row < 210)
      || (column >= 260 && column < 305 && row >= 210 && row < 225)
    const color = letter ? [32, 85, 128] : bubble || tail ? [248, 251, 255] : [32, 85, 128]
    const offset = row * (width * 3 + 1) + 1 + column * 3
    pixels.set(color, offset)
  }
}
function chunk(kind, content) {
  const type = Buffer.from(kind)
  const data = Buffer.concat([type, content])
  let checksum = 0xffffffff
  for (const byte of data) {
    checksum ^= byte
    for (let bit = 0; bit < 8; bit++) checksum = (checksum >>> 1) ^ ((checksum & 1) ? 0xedb88320 : 0)
  }
  const size = Buffer.alloc(4)
  size.writeUInt32BE(content.length)
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE((checksum ^ 0xffffffff) >>> 0)
  return Buffer.concat([size, data, crc])
}
const header = Buffer.alloc(13)
header.writeUInt32BE(width, 0)
header.writeUInt32BE(height, 4)
header[8] = 8
header[9] = 2
writeFileSync(new URL('../src/static/images/share-cover.png', import.meta.url), Buffer.concat([
  Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
  chunk('IHDR', header), chunk('IDAT', deflateSync(pixels)), chunk('IEND', Buffer.alloc(0)),
]))
