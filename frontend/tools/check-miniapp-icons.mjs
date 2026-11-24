import { createHash } from 'node:crypto'
import { existsSync, lstatSync, readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { inflateSync } from 'node:zlib'

export const miniappRoot = fileURLToPath(new URL('../apps/miniapp/', import.meta.url))
const filenamePattern = /^[a-z0-9]+(?:-[a-z0-9]+)*\.png$/
const digestPattern = /^[a-f0-9]{64}$/
export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex')

function crc32(bytes) {
  let checksum = 0xffffffff
  for (const byte of bytes) {
    checksum ^= byte
    for (let bit = 0; bit < 8; bit++) checksum = (checksum >>> 1) ^ ((checksum & 1) ? 0xedb88320 : 0)
  }
  return (checksum ^ 0xffffffff) >>> 0
}

export function pngDimensions(bytes) {
  if (bytes.length < 57 || !bytes.subarray(0, 8).equals(Buffer.from('89504e470d0a1a0a', 'hex'))) throw new Error('invalid PNG signature')
  let offset = 8
  let dimensions
  let ended = false
  const compressed = []
  while (offset + 12 <= bytes.length) {
    const length = bytes.readUInt32BE(offset)
    const end = offset + 12 + length
    if (end > bytes.length) throw new Error('truncated PNG chunk')
    const type = bytes.toString('ascii', offset + 4, offset + 8)
    if (crc32(bytes.subarray(offset + 4, end - 4)) !== bytes.readUInt32BE(end - 4)) throw new Error('PNG CRC mismatch')
    if (offset === 8 && type !== 'IHDR') throw new Error('PNG must start with IHDR')
    if (type === 'IHDR') {
      if (dimensions || length !== 13) throw new Error('invalid PNG header')
      const width = bytes.readUInt32BE(offset + 8), height = bytes.readUInt32BE(offset + 12)
      if (!width || !height || width > 512 || height > 512 || !bytes.subarray(offset + 16, offset + 21).equals(Buffer.from([8, 6, 0, 0, 0]))) throw new Error('expected bounded non-interlaced RGBA8 PNG')
      dimensions = { width, height }
    } else if (type === 'IDAT') compressed.push(bytes.subarray(offset + 8, end - 4))
    else if (type === 'IEND') {
      if (length || end !== bytes.length) throw new Error('invalid PNG ending')
      ended = true
      break
    } else throw new Error(`unexpected compressed PNG chunk: ${type}`)
    offset = end
  }
  if (!dimensions || !ended || !compressed.length) throw new Error('incomplete PNG')
  const rowBytes = dimensions.width * 4 + 1
  const raw = inflateSync(Buffer.concat(compressed), { maxOutputLength: rowBytes * dimensions.height })
  if (raw.length !== rowBytes * dimensions.height) throw new Error('invalid PNG pixel data length')
  for (let row = 0; row < dimensions.height; row++) if (raw[row * rowBytes] > 4) throw new Error('invalid PNG filter')
  return dimensions
}

export function validateAssets(manifest, directory) {
  if (manifest.schema_version !== 1 || !manifest.tool || !manifest.parameters || !manifest.source || !Array.isArray(manifest.assets) || !manifest.assets.length) throw new Error('invalid icon manifest')
  const buffers = new Map()
  let originalTotal = 0, compressedTotal = 0
  for (const asset of manifest.assets) {
    if (!filenamePattern.test(asset.file) || buffers.has(asset.file)) throw new Error(`invalid or duplicate icon filename: ${asset.file}`)
    if (!Number.isSafeInteger(asset.original_bytes) || !Number.isSafeInteger(asset.compressed_bytes) || asset.compressed_bytes <= 0 || asset.compressed_bytes >= asset.original_bytes || asset.rgba_identical !== true || !asset.filter || !digestPattern.test(asset.source_sha256) || !digestPattern.test(asset.sha256) || asset.source_sha256 === asset.sha256) throw new Error(`missing strict lossless compression evidence: ${asset.file}`)
    const file = path.join(directory, asset.file)
    if (!lstatSync(file).isFile()) throw new Error(`icon must be a regular file: ${asset.file}`)
    const bytes = readFileSync(file)
    const dimensions = pngDimensions(bytes)
    if (bytes.length !== asset.compressed_bytes || sha256(bytes) !== asset.sha256 || dimensions.width !== asset.width || dimensions.height !== asset.height) throw new Error(`icon differs from compression manifest: ${asset.file}`)
    originalTotal += asset.original_bytes
    compressedTotal += asset.compressed_bytes
    buffers.set(asset.file, bytes)
  }
  if (originalTotal !== manifest.original_bytes || compressedTotal !== manifest.compressed_bytes) throw new Error('icon compression totals do not match')
  return buffers
}

export function validateMapping(mapping, assets) {
  const used = new Set()
  for (const [name, pair] of Object.entries(mapping)) {
    if (!Array.isArray(pair) || pair.length !== 2) throw new Error(`missing icon state pair: ${name}`)
    for (const filename of pair) {
      if (!assets.has(filename)) throw new Error(`unregistered icon: ${filename}`)
      used.add(filename)
    }
  }
  for (const filename of assets.keys()) if (!used.has(filename)) throw new Error(`unused icon: ${filename}`)
  for (const name of ['home', 'following', 'messages', 'profile']) {
    if (!mapping[name] || mapping[name][0] === mapping[name][1]) throw new Error(`navigation needs distinct states: ${name}`)
  }
  if (mapping.publish?.some(filename => filename !== 'add-white.png') || mapping.publish?.length !== 2) throw new Error('publish icon must use the white plus on its colored button')
}

export function checkIcons(root = miniappRoot, buildDirectory) {
  const manifest = JSON.parse(readFileSync(path.join(root, 'icon-assets.manifest.json'), 'utf8'))
  const directory = path.join(root, 'src/static/icons')
  const assets = validateAssets(manifest, directory)
  if (!manifest.review?.raster_comparison || !manifest.review?.date) throw new Error('missing raster comparison review')
  const actual = readdirSync(directory).sort()
  if (JSON.stringify(actual) !== JSON.stringify([...assets.keys()].sort())) throw new Error('unregistered files or original backups in static/icons')
  validateMapping(JSON.parse(readFileSync(path.join(root, 'src/shared/icon-assets.json'), 'utf8')), assets)
  const selection = JSON.parse(readFileSync(new URL('../../tools/miniapp-icons-selection.json', import.meta.url), 'utf8'))
  if (JSON.stringify([...selection.files].sort()) !== JSON.stringify([...assets.keys()].sort())) throw new Error('selection and migrated assets differ')
  const config = JSON.parse(readFileSync(path.join(root, 'src/pages.json'), 'utf8'))
  for (const item of config.tabBar?.list ?? []) {
    for (const key of ['iconPath', 'selectedIconPath']) {
      if (item[key] && !assets.has(item[key].replace(/^\/?static\/icons\//, ''))) throw new Error(`unregistered Tab icon: ${item[key]}`)
    }
  }
  if (buildDirectory) {
    for (const [filename, bytes] of assets) {
      const output = path.join(buildDirectory, 'static/icons', filename)
      if (!existsSync(output) || !readFileSync(output).equals(bytes)) throw new Error(`missing or changed build icon: ${filename}`)
    }
    if (JSON.stringify(readdirSync(path.join(buildDirectory, 'static/icons')).sort()) !== JSON.stringify([...assets.keys()].sort())) throw new Error('unexpected build icon resources')
  }
  return { count: assets.size, originalBytes: manifest.original_bytes, compressedBytes: manifest.compressed_bytes }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = checkIcons(miniappRoot, process.argv[2])
  console.log(`图标检查通过：${result.count} 个，${result.originalBytes} → ${result.compressedBytes} 字节。`)
}
