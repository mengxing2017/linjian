import assert from 'node:assert/strict'
import { afterEach, beforeEach, test } from 'node:test'
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync, existsSync, symlinkSync, copyFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import vm from 'node:vm'
import { checkIcons, miniappRoot, pngDimensions, sha256, validateAssets, validateMapping } from './check-miniapp-icons.mjs'
import { importIcons } from './import-miniapp-icons.mjs'

const manifest = JSON.parse(readFileSync(path.join(miniappRoot, 'icon-assets.manifest.json'), 'utf8'))
const mapping = JSON.parse(readFileSync(path.join(miniappRoot, 'src/shared/icon-assets.json'), 'utf8'))
const selection = { files: manifest.assets.map(asset => asset.file) }
const fixture = manifest.assets[0]
const fixtureBytes = readFileSync(path.join(miniappRoot, 'src/static/icons', fixture.file))
let temporary
beforeEach(() => { temporary = mkdtempSync(path.join(os.tmpdir(), 'linjian-icon-check-')) })
afterEach(() => rmSync(temporary, { recursive: true, force: true }))

function singleManifest() {
  return { ...structuredClone(manifest), assets: [structuredClone(fixture)], original_bytes: fixture.original_bytes, compressed_bytes: fixture.compressed_bytes }
}
function stage() {
  const directory = path.join(temporary, 'staging')
  mkdirSync(directory)
  for (const asset of manifest.assets) copyFileSync(path.join(miniappRoot, 'src/static/icons', asset.file), path.join(directory, asset.file))
  writeFileSync(path.join(directory, 'manifest.json'), JSON.stringify(manifest))
  return {
    staging: directory,
    destination: path.join(temporary, 'target/icons'),
    manifestFile: path.join(temporary, 'target/manifest.json'),
    mapping,
    selection,
    reviewed: true,
  }
}

test('committed assets are strictly smaller, registered, pixel-verified and self-contained', () => {
  const result = checkIcons()
  assert.equal(result.count, 32)
  assert.ok(result.compressedBytes < result.originalBytes)
  for (const asset of manifest.assets) assert.ok(asset.compressed_bytes < asset.original_bytes)
})

test('PNG checks validate dimensions, CRC, ending and compressed pixels', () => {
  assert.deepEqual(pngDimensions(fixtureBytes), { width: fixture.width, height: fixture.height })
  for (const bytes of [Buffer.alloc(0), Buffer.from('not png'), fixtureBytes.subarray(0, -4), Buffer.concat([fixtureBytes, Buffer.from('backup')])]) assert.throws(() => pngDimensions(bytes))
  const damaged = Buffer.from(fixtureBytes)
  damaged[45] ^= 1
  assert.throws(() => pngDimensions(damaged), /CRC/)
})

test('rejects unchanged, larger or unverified compression records', () => {
  writeFileSync(path.join(temporary, fixture.file), fixtureBytes)
  for (const changes of [
    { original_bytes: fixture.compressed_bytes },
    { original_bytes: fixture.compressed_bytes - 1 },
    { rgba_identical: false },
    { sha256: fixture.source_sha256 },
    { source_sha256: 'unknown' },
    { compressed_bytes: 0 },
    { width: 48 },
  ]) {
    const value = singleManifest()
    Object.assign(value.assets[0], changes)
    assert.throws(() => validateAssets(value, temporary))
  }
})

test('rejects traversal, duplicate records, missing icons and symlinks', () => {
  const value = singleManifest()
  value.assets[0].file = '../home.png'
  assert.throws(() => validateAssets(value, temporary), /filename/)
  assert.throws(() => validateAssets(singleManifest(), temporary))
  symlinkSync(path.join(miniappRoot, 'src/static/icons', fixture.file), path.join(temporary, fixture.file))
  assert.throws(() => validateAssets(singleManifest(), temporary), /regular file/)
})

test('rejects duplicates, hash changes and incorrect totals', () => {
  writeFileSync(path.join(temporary, fixture.file), fixtureBytes)
  const duplicate = singleManifest()
  duplicate.assets.push(structuredClone(fixture))
  assert.throws(() => validateAssets(duplicate, temporary), /duplicate/)
  const altered = singleManifest()
  altered.assets[0].sha256 = '0'.repeat(64)
  assert.throws(() => validateAssets(altered, temporary), /differs/)
  const totals = singleManifest()
  totals.original_bytes++
  assert.throws(() => validateAssets(totals, temporary), /totals/)
})

test('mapping enforces registered resources, full pairs, distinct navigation states and no unused assets', () => {
  const assets = new Map(manifest.assets.map(asset => [asset.file, true]))
  assert.doesNotThrow(() => validateMapping(mapping, assets))
  assert.throws(() => validateMapping({ ...mapping, home: ['missing.png', 'missing.png'] }, assets), /unregistered/)
  assert.throws(() => validateMapping({ ...mapping, home: ['home-neutral.png'] }, assets), /pair/)
  assert.throws(() => validateMapping({ ...mapping, extra: ['../bad.png', '../bad.png'] }, assets), /unregistered/)
  assert.throws(() => validateMapping({ ...mapping, publish: ['home-neutral.png', 'add-white.png'] }, assets), /white plus/)
  assert.throws(() => validateMapping(mapping, new Map([...assets, ['unused.png', true]])), /unused/)
})

test('reviewed import copies only validated compressed buffers, never originals or previews', () => {
  const options = stage()
  writeFileSync(path.join(options.staging, 'preview.png'), 'not an icon')
  assert.equal(importIcons(options), 32)
  assert.ok(!existsSync(path.join(options.destination, 'preview.png')))
  const imported = JSON.parse(readFileSync(options.manifestFile, 'utf8'))
  assert.equal(validateAssets(imported, options.destination).size, 32)
  assert.ok(imported.review.device_review.startsWith('Pending'))
})

test('unreviewed, tampered or unselected stages fail without creating a destination', () => {
  const options = stage()
  assert.throws(() => importIcons({ ...options, reviewed: false }), /review/)
  assert.ok(!existsSync(options.destination))
  assert.throws(() => importIcons({ ...options, selection: { files: [] } }), /selection/)
  writeFileSync(path.join(options.staging, fixture.file), Buffer.from('original fallback forbidden'))
  assert.throws(() => importIcons(options))
  assert.ok(!existsSync(options.destination))
})

test('import refuses to overwrite existing files and rolls back its own new files on failure', () => {
  const options = stage()
  mkdirSync(path.dirname(options.destination))
  writeFileSync(options.manifestFile, 'user changes')
  assert.throws(() => importIcons(options), /already exists/)
  assert.equal(readFileSync(options.manifestFile, 'utf8'), 'user changes')
  assert.ok(!existsSync(options.destination))
  assert.throws(() => importIcons({ ...options, manifestFile: path.join(temporary, 'missing/manifest.json') }))
  assert.ok(!existsSync(options.destination))
})

test('build checks reject missing or changed output assets', () => {
  const build = path.join(temporary, 'build')
  mkdirSync(path.join(build, 'static/icons'), { recursive: true })
  assert.throws(() => checkIcons(miniappRoot, build), /build icon/)
  for (const asset of manifest.assets) copyFileSync(path.join(miniappRoot, 'src/static/icons', asset.file), path.join(build, 'static/icons', asset.file))
  assert.equal(checkIcons(miniappRoot, build).count, 32)
  writeFileSync(path.join(build, 'static/icons', fixture.file), 'changed')
  assert.throws(() => checkIcons(miniappRoot, build), /changed build icon/)
})

test('native icon errors only mark registered sources, leaving navigation available', () => {
  let component
  vm.runInNewContext(readFileSync(path.join(miniappRoot, 'src/custom-tab-bar/index.js'), 'utf8'), { Component: value => { component = value } })
  const updates = []
  const bar = { data: { items: [{ icon: '/static/icons/home-neutral.png', selectedIcon: '/static/icons/home-primary.png' }], failedSources: {} }, setData: data => updates.push(data) }
  component.methods.iconError.call(bar, { currentTarget: { dataset: { source: 'https://unknown.example/icon.png' } } })
  assert.equal(updates.length, 0)
  component.methods.iconError.call(bar, { currentTarget: { dataset: { source: '/static/icons/home-neutral.png' } } })
  assert.equal(updates[0].failedSources['/static/icons/home-neutral.png'], true)
  assert.equal(sha256(fixtureBytes), fixture.sha256)
})
