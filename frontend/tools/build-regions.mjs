import fs from 'node:fs'
import path from 'node:path'
import zlib from 'node:zlib'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const packageRoot = path.join(root, 'packages/china-regions')
const sourceRoot = path.join(packageRoot, 'data/source')
const output = path.join(packageRoot, 'data/regions.json')

const data = JSON.parse(fs.readFileSync(path.join(sourceRoot, 'pca-code.json'), 'utf8'))
const hmt = JSON.parse(fs.readFileSync(path.join(sourceRoot, 'HK-MO-TW.json'), 'utf8'))

const municipalities = new Set(['北京市', '天津市', '上海市', '重庆市'])
const hmtCodes = {
  香港特别行政区: '810000',
  澳门特别行政区: '820000',
  台湾省: '710000',
}

const provinces = data.map((province) => ({
  code: province.code,
  name: province.name,
  children: (province.children ?? []).map((city) => ({
    code: city.code,
    name: municipalities.has(province.name) && city.name === '市辖区' ? province.name : city.name,
    children: (city.children ?? []).map((area) => ({
      code: area.code,
      name: area.name,
    })),
  })),
}))

for (const [provinceName, cities] of Object.entries(hmt)) {
  const provinceCode = hmtCodes[provinceName]
  if (!provinceCode) throw new Error(`Missing codomain for ${provinceName}`)
  provinces.push({
    code: provinceCode,
    name: provinceName,
    children: Object.entries(cities).map(([cityName, areas], cityIndex) => {
      const cityCode = `${provinceCode.slice(0, 2)}${String(cityIndex + 1).padStart(2, '0')}`
      return {
        code: cityCode,
        name: cityName,
        children: areas.map((areaName, areaIndex) => ({
          code: `${cityCode}${String(areaIndex + 1).padStart(2, '0')}`,
          name: areaName,
        })),
      }
    }),
  })
}

function walk(nodes, visit) {
  for (const node of nodes) {
    visit(node)
    if (node.children) walk(node.children, visit)
  }
}

const codes = new Set()
let cityCount = 0
let areaCount = 0
walk(provinces, (node) => {
  if (!node.name || !node.name.trim()) throw new Error(`Empty region name: ${node.code}`)
  if (codes.has(node.code)) throw new Error(`Duplicate region code: ${node.code}`)
  codes.add(node.code)
})

for (const province of provinces) {
  for (const city of province.children ?? []) {
    cityCount += 1
    if (!city.code.startsWith(province.code.slice(0, 2))) {
      throw new Error(`City code ${city.code} does not belong to province ${province.code}`)
    }
    for (const area of city.children ?? []) {
      areaCount += 1
      if (!area.code.startsWith(city.code)) {
        throw new Error(`Area code ${area.code} does not belong to city ${city.code}`)
      }
    }
  }
}

if (provinces.length < 34) throw new Error(`Unexpected province count: ${provinces.length}`)
if (cityCount < 300) throw new Error(`Unexpected city count: ${cityCount}`)
if (areaCount < 2000) throw new Error(`Unexpected area count: ${areaCount}`)

const dataset = {
  version: 'china-division@2.7.0',
  source: 'china-division pca-code.json + HK-MO-TW.json',
  provinces,
}

const json = `${JSON.stringify(dataset)}\n`
fs.writeFileSync(output, json)
const raw = Buffer.byteLength(json)
const gzip = zlib.gzipSync(json).byteLength
console.log(`provinces=${provinces.length} cities=${cityCount} areas=${areaCount}`)
console.log(`raw=${raw} bytes gzip=${gzip} bytes`)
console.log(`wrote ${path.relative(root, output)}`)
