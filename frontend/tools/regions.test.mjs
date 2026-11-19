import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import zlib from 'node:zlib'
import test from 'node:test'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const file = path.join(root, 'packages/china-regions/data/regions.json')
const dataset = JSON.parse(fs.readFileSync(file, 'utf8'))

function walk(nodes, visit) {
  for (const node of nodes) {
    visit(node)
    if (node.children) walk(node.children, visit)
  }
}

test('region dataset has complete three-level structure', () => {
  assert.match(dataset.version, /\S/)
  assert.match(dataset.source, /\S/)
  assert.ok(dataset.provinces.length >= 34)

  let cities = 0
  let areas = 0
  for (const province of dataset.provinces) {
    assert.ok(province.code && province.name)
    assert.ok(Array.isArray(province.children) && province.children.length > 0)
    for (const city of province.children) {
      cities += 1
      assert.ok(city.code && city.name)
      assert.ok(city.code.startsWith(province.code.slice(0, 2)))
      assert.ok(Array.isArray(city.children) && city.children.length > 0)
      for (const area of city.children) {
        areas += 1
        assert.ok(area.code && area.name)
        assert.ok(area.code.startsWith(city.code))
      }
    }
  }

  assert.ok(cities >= 300)
  assert.ok(areas >= 2000)
})

test('region codes and names are unique and valid', () => {
  const codes = new Set()
  walk(dataset.provinces, (node) => {
    assert.equal(typeof node.code, 'string')
    assert.ok(node.code.length > 0)
    assert.ok(node.name.trim().length > 0)
    assert.ok(!codes.has(node.code), `duplicate code ${node.code}`)
    codes.add(node.code)
  })
})

test('region dataset stays within the bundle budget', () => {
  const raw = fs.readFileSync(file)
  assert.ok(raw.byteLength <= 500 * 1024, `raw dataset is ${raw.byteLength} bytes`)
  const gzip = zlib.gzipSync(raw).byteLength
  assert.ok(gzip <= 120 * 1024, `gzipped dataset is ${gzip} bytes`)
})
