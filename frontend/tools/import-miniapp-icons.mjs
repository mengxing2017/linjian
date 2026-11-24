import { existsSync, mkdirSync, readFileSync, readdirSync, realpathSync, rmdirSync, unlinkSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { miniappRoot, validateAssets, validateMapping } from './check-miniapp-icons.mjs'

export function importIcons({ staging, destination, manifestFile, mapping, selection, reviewed }) {
  if (!reviewed) throw new Error('review the paired raster preview before importing; --reviewed is required')
  const temporary = `${realpathSync(os.tmpdir())}${path.sep}`
  if (!realpathSync(staging).startsWith(temporary)) throw new Error('staging must be under the system temporary directory')
  const manifest = JSON.parse(readFileSync(path.join(staging, 'manifest.json'), 'utf8'))
  const buffers = validateAssets(manifest, staging)
  validateMapping(mapping, buffers)
  if (JSON.stringify([...selection.files].sort()) !== JSON.stringify([...buffers.keys()].sort())) throw new Error('selection does not match staged assets')
  if (existsSync(manifestFile) || existsSync(destination)) throw new Error('destination already exists; never overwrite existing assets')
  const review = {
    date: new Date().toISOString().slice(0, 10),
    raster_comparison: 'Original/compressed pairs reviewed on white, dark and blue backgrounds in staging preview.png; RGBA pixels identical.',
    device_review: 'Pending WeChat developer tools and iOS/Android screenshots; not a device acceptance claim.',
  }
  mkdirSync(path.dirname(destination), { recursive: true })
  mkdirSync(destination)
  const created = []
  try {
    for (const [filename, bytes] of buffers) {
      const file = path.join(destination, filename)
      writeFileSync(file, bytes, { flag: 'wx' })
      created.push({ file, bytes })
    }
    writeFileSync(manifestFile, `${JSON.stringify({ ...manifest, review }, null, 2)}\n`, { flag: 'wx' })
  } catch (cause) {
    for (const { file, bytes } of created) {
      if (existsSync(file) && readFileSync(file).equals(bytes)) unlinkSync(file)
    }
    if (!readdirSync(destination).length) rmdirSync(destination)
    throw cause
  }
  return buffers.size
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const staging = process.argv[2]
  if (!staging) throw new Error('usage: node frontend/tools/import-miniapp-icons.mjs <staging> --reviewed')
  const count = importIcons({
    staging,
    destination: path.join(miniappRoot, 'src/static/icons'),
    manifestFile: path.join(miniappRoot, 'icon-assets.manifest.json'),
    mapping: JSON.parse(readFileSync(path.join(miniappRoot, 'src/shared/icon-assets.json'), 'utf8')),
    selection: JSON.parse(readFileSync(new URL('../../tools/miniapp-icons-selection.json', import.meta.url), 'utf8')),
    reviewed: process.argv.includes('--reviewed'),
  })
  console.log(`迁入 ${count} 个已压缩且验收通过的图标；未复制原图。`)
}
